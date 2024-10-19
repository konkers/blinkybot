//! This example shows how to communicate asynchronous using i2c with external chips.
//!
//! Example written for the [`MCP23017 16-Bit I2C I/O Expander with Serial Interface`] chip.
//! (https://www.microchip.com/en-us/product/mcp23017)

#![no_std]
#![no_main]

use blinkybot_rpc::ExpressionIndex;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join;
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_rp::adc::{self, Adc, Channel};
use embassy_rp::bind_interrupts;
use embassy_rp::block::ImageDef;
use embassy_rp::flash::{Async, Flash};
use embassy_rp::gpio::Pull;
use embassy_rp::i2c::{self, Config};
use embassy_rp::peripherals::{I2C1, USB};
use embassy_rp::usb;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel;
use embassy_sync::watch::DynReceiver;
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_hal_async::i2c::I2c;
use is31fl3731_async::devices::CharlieWing;
use is31fl3731_async::IS31FL3731;
use oorandom::Rand32;
use postcard::fixint::be;
use webusb::Comms;
use {defmt_rtt as _, panic_probe as _};

mod config_store;
mod error;
mod webusb;

pub use error::Error;
pub use error::Result;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"example"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_description!(c"Blinky"),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

bind_interrupts!(struct Irqs {
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

macro_rules! symbol_address {
    ($symbol:ident) => {{
        extern "C" {
            static $symbol: u32;
        }
        unsafe { (&$symbol) as *const u32 as u32 }
    }};
}
const FLASH_SIZE: usize = 8 * 1024 * 1024;

#[embassy_executor::main]
async fn main_(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let sda = p.PIN_2;
    let scl = p.PIN_3;

    info!("set up config storage");
    let flash_start = symbol_address!(_flash_start);
    let user_flash_start = symbol_address!(_user_flash_start);
    let user_flash_end = symbol_address!(_user_flash_end);
    let flash_range = (user_flash_start - flash_start)..(user_flash_end - flash_start);

    defmt::assert_eq!((user_flash_end - flash_start) as usize, FLASH_SIZE);

    let flash = Flash::<_, Async, FLASH_SIZE>::new(p.FLASH, p.DMA_CH0);
    let config_store = config_store::FlashConfigStore::new(flash, flash_range);

    info!("set up ADC");
    let mut adc = Adc::new(p.ADC, Irqs, adc::Config::default());
    let sense_adc = Channel::new_pin(p.PIN_28, Pull::Up);

    info!("set up comms");
    // Create the driver, from the HAL.
    let driver = usb::Driver::new(p.USB, Irqs);
    let comms = webusb::setup(spawner, driver, config_store).await;

    info!("set up i2c ");
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, Config::default());

    info!("Setting up display");
    let mut matrix = CharlieWing::configure(i2c);
    unwrap!(matrix.setup(&mut Delay {}).await, "Failed to setup display");

    info!("starting coroutines");
    let adc_fut = adc_sampler(comms, adc, sense_adc);
    let behavior_fut = behavior(matrix, comms);

    info!("joining");
    join::join(adc_fut, behavior_fut).await;
    //behavior_fut.await;
    error!("reached end of main");
}

async fn adc_sampler(comms: &Comms, mut adc: Adc<'_, adc::Async>, mut input: Channel<'_>) -> ! {
    let sender = comms.adc_val.dyn_sender();
    loop {
        let val = adc.read(&mut input).await.unwrap();
        sender.send(val);
        Timer::after_millis(100).await;
    }
}

fn is_friend(val: u16) -> bool {
    val < 0x100
}

async fn behavior<I2C, I2cError>(mut matrix: IS31FL3731<I2C>, comms: &Comms) -> !
where
    I2C: I2c<Error = I2cError>,
{
    let mut default_expression = comms.default_expression.dyn_receiver().unwrap();
    let mut blink_expression = comms.blink_expression.dyn_receiver().unwrap();
    let mut friend_expression = comms.friend_expression.dyn_receiver().unwrap();
    let mut friend_blink_expression = comms.friend_blink_expression.dyn_receiver().unwrap();
    let mut adc_val_receiver = comms.adc_val.dyn_receiver().unwrap();

    let mut rng = Rand32::new(0);
    let mut seeing_friend = is_friend(adc_val_receiver.get().await);

    set_face(&mut matrix, default_expression.get().await.pixels).await;
    loop {
        let until = Instant::now() + Duration::from_millis(rng.rand_range(2000..10000).into());
        while Instant::now() < until {
            let fut = Timer::at(until);
            match select(fut, adc_val_receiver.changed()).await {
                Either::First(_) => break,
                Either::Second(val) => {
                    seeing_friend = is_friend(val);
                }
            }

            if seeing_friend {
                set_face(&mut matrix, friend_expression.get().await.pixels).await;
            } else {
                set_face(&mut matrix, default_expression.get().await.pixels).await;
            }
        }
        info!("blink");
        if seeing_friend {
            set_face(&mut matrix, friend_blink_expression.get().await.pixels).await;
        } else {
            set_face(&mut matrix, blink_expression.get().await.pixels).await;
        }
        Timer::after_millis(25).await;
    }
}

async fn set_face<I2C, I2cError>(matrix: &mut IS31FL3731<I2C>, face: [u16; 7])
where
    I2C: I2c<Error = I2cError>,
{
    for (y, row) in face.iter().enumerate() {
        for x in 0..15 {
            let value = if (row & (1 << x)) != 0 { 0x2f } else { 0x00 };
            unwrap!(
                matrix.pixel(x, y as u8, value).await,
                "Failed to set pixel light on"
            );
        }
    }
}
