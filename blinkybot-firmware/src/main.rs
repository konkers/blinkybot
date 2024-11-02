//! This example test the RP Pico on board LED.
//!
//! It does not work with the RP Pico W board. See wifi_blinky.rs.

#![no_std]
#![no_main]
#![feature(wrapping_int_impl)]

use core::{borrow::BorrowMut, num::Wrapping, ops::DerefMut};
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::{join, join3};
use embassy_rp::gpio;
use embassy_rp::i2c::InterruptHandler;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::{DynReceiver, DynSender, Watch};
use embassy_time::{Delay, Timer};
use embedded_hal_async::i2c::I2c;
use gpio::{Level, Output};
use is31fl3731_async::{devices::CharlieWing, IS31FL3731};
use oorandom::Rand32;
use {defmt_rtt as _, panic_probe as _};

type Mutex<T> = embassy_sync::mutex::Mutex<ThreadModeRawMutex, T>;

mod as1115;

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<embassy_rp::peripherals::I2C0>;
    I2C1_IRQ => InterruptHandler<embassy_rp::peripherals::I2C1>;
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
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

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
