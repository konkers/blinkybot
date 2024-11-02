//! This example test the RP Pico on board LED.
//!
//! It does not work with the RP Pico W board. See wifi_blinky.rs.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::ops::DerefMut;
use cyw43::Control;
use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join4;
use embassy_futures::join::{join, join3};
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_net::StackResources;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio;
use embassy_rp::i2c;
use embassy_rp::peripherals::DMA_CH0;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{self, Pio};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::{DynReceiver, DynSender, Watch};
use embassy_time::Duration;
use embassy_time::{Delay, Timer};
use embedded_hal_async::i2c::I2c;
use embedded_io_async::Write;
use gpio::{Level, Output};
use is31fl3731_async::{devices::CharlieWing, IS31FL3731};
use oorandom::Rand32;
use picoserve::Router;
use rand::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

type Mutex<T> = embassy_sync::mutex::Mutex<ThreadModeRawMutex, T>;

mod as1115;

#[macro_export]
macro_rules! make_static {
    ($ty:ty, $val:expr) => {{
        static STATIC_CELL: $crate::StaticCell<$ty> = $crate::StaticCell::new();
        STATIC_CELL.init($val)
    }};
}

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
    I2C1_IRQ => i2c::InterruptHandler<embassy_rp::peripherals::I2C1>;

    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
});

async fn set_frame<I2C, I2cError>(
    matrix: &mut IS31FL3731<I2C, I2cError>,
    i2c: &mut I2C,
    frame: [u16; 7],
    brightness: u8,
) where
    I2C: I2c<Error = I2cError>,
{
    for (y, row) in frame.iter().enumerate() {
        for x in 0..15 {
            let value = if (row & (1 << x)) != 0 {
                brightness
            } else {
                0x00
            };
            unwrap!(
                matrix.pixel(i2c, x, y as u8, value).await,
                "Failed to set pixel light on"
            );
        }
    }
}
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut rng = RoscRng;

    let fw = include_bytes!("../../third_party/embassy/cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../third_party/embassy/cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Use a link-local address for communication without DHCP server
    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(169, 254, 1, 1), 16),
        dns_servers: heapless::Vec::new(),
        gateway: None,
    });

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<12>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    //control.start_ap_open("cyw43", 5).await;
    control.start_ap_wpa2("cyw43", "password", 5).await;

    let config = embassy_rp::i2c::Config::default();
    let i2c0 = Mutex::new(embassy_rp::i2c::I2c::new_async(
        p.I2C0, /* SCL */ p.PIN_1, /* SDA */ p.PIN_0, Irqs, config,
    ));
    let i2c1 = Mutex::new(embassy_rp::i2c::I2c::new_async(
        p.I2C1, /* SCL */ p.PIN_27, /* SDA */ p.PIN_26, Irqs, config,
    ));

    let wheel_watch = Watch::<ThreadModeRawMutex, u8, 1>::new();

    let wheel_fut = wheel(&i2c0, wheel_watch.dyn_sender());
    let face_fut = face(&i2c1);
    let matrix_fut = matrix_petal(&i2c1, wheel_watch.dyn_receiver().unwrap());

    let app = make_static!(Router<approuter::AppRouter>, approuter::make_app());

    let config = make_static!(
        picoserve::Config<Duration>,
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            read_request: Some(Duration::from_secs(1)),
            write: Some(Duration::from_secs(1)),
        })
        .keep_connection_alive()
    );

    for id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(id, stack, app, config));
    }

    join3(wheel_fut, face_fut, matrix_fut).await;
    loop {}
}

async fn wheel<'a, I2C, I2cError>(i2c: &Mutex<I2C>, wheel: DynSender<'a, u8>) -> !
where
    I2C: I2c<Error = I2cError>,
    I2cError: Format,
{
    loop {
        let val = {
            let mut i2c = i2c.lock().await;
            let mut buf = [0u8; 1];
            unwrap!(i2c.write_read(0x54, &[0x0], &mut buf).await);
            buf[0]
        };
        wheel.send(val);
    }
}

async fn face<I2C, I2cError>(i2c: &Mutex<I2C>) -> !
where
    I2C: I2c<Error = I2cError>,
    I2cError: Format,
{
    let face = [
        0b000_0000_0000_0000,
        0b001_1000_0000_1100,
        0b010_0100_0001_0010,
        0b010_0100_0001_0010,
        0b001_1001_0100_1100,
        0b000_0001_1100_0000,
        0b000_0000_0000_0000,
    ];
    let blink = [
        0b000_0000_0000_0000,
        0b000_0000_0000_0000,
        0b011_1100_0001_1110,
        0b000_0000_0000_0000,
        0b000_0001_0100_0000,
        0b000_0001_1100_0000,
        0b000_0000_0000_0000,
    ];
    let mut rng = Rand32::new(0);
    info!("Setting up display");
    let mut charlie_matrix = CharlieWing::configure();
    {
        let mut i2c = i2c.lock().await;
        unwrap!(
            charlie_matrix.setup(i2c.deref_mut(), &mut Delay {}).await,
            "Failed to setup display"
        );
        set_frame(&mut charlie_matrix, i2c.deref_mut(), face, 0xff).await;
    }
    loop {
        {
            let mut i2c = i2c.lock().await;
            set_frame(&mut charlie_matrix, i2c.deref_mut(), face, 0xff).await;
        }
        Timer::after_millis(rng.rand_range(2000..10000).into()).await;
        {
            let mut i2c = i2c.lock().await;
            set_frame(&mut charlie_matrix, i2c.deref_mut(), blink, 0xff).await;
        }
        Timer::after_millis(25).await;
    }
}

async fn matrix_petal<'a, I2C, I2cError>(i2c: &Mutex<I2C>, mut wheel: DynReceiver<'a, u8>) -> !
where
    I2C: I2c<Error = I2cError>,
    I2cError: Format,
{
    let mut matrix = as1115::As1115::new();
    {
        let mut i2c = i2c.lock().await;
        info!("init");
        matrix.init(i2c.deref_mut()).await;
    }
    loop {
        for led in 0..=6 {
            let wheel_val = wheel.try_get().unwrap_or(0);
            let led_val = if wheel_val > 0 { wheel_val >> 5 } else { led };
            matrix.clear();
            for arm in 0..=7 {
                matrix.set_arm(arm, led_val, true);
            }
            {
                let mut i2c = i2c.lock().await;
                matrix.sync(i2c.deref_mut()).await;
            }
            Timer::after_millis(100).await;
        }
    }
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

async fn server<'a>(stack: Stack<'a>, mut control: Control<'a>) -> ! {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;
        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("read error: {:?}", e);
                    break;
                }
            };

            info!("rxd {}", core::str::from_utf8(&buf[..n]).unwrap());

            match socket.write_all(&buf[..n]).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("write error: {:?}", e);
                    break;
                }
            };
        }
    }
}

mod approuter {
    use picoserve::routing::get;

    pub type AppRouter = impl picoserve::routing::PathRouter;
    pub fn make_app() -> picoserve::Router<AppRouter> {
        picoserve::Router::new().route("/", get(|| async move { "Hello World" }))
    }
}

const WEB_TASK_POOL_SIZE: usize = 8;

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static picoserve::Router<approuter::AppRouter>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::listen_and_serve(
        id,
        app,
        config,
        stack,
        port,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    )
    .await
}
