//! This example shows how to communicate asynchronous using i2c with external chips.
//!
//! Example written for the [`MCP23017 16-Bit I2C I/O Expander with Serial Interface`] chip.
//! (https://www.microchip.com/en-us/product/mcp23017)

#![no_std]
#![no_main]

use blinkybot_rpc::ExpressionIndex;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::block::ImageDef;
use embassy_rp::flash::{Async, Flash};
use embassy_rp::i2c::{self, Config};
use embassy_rp::peripherals::{I2C1, USB};
use embassy_rp::usb;
use embassy_time::{Delay, Timer};
use embedded_hal_async::i2c::I2c;
use is31fl3731_async::devices::CharlieWing;
use is31fl3731_async::IS31FL3731;
use oorandom::Rand32;
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

    // let default_expression = config_store.get_expression(ExpressionIndex::Default).await;
    // let blink_expression = config_store.get_expression(ExpressionIndex::Blink).await;

    info!("set up comms");
    // Create the driver, from the HAL.
    let driver = usb::Driver::new(p.USB, Irqs);
    let comms = webusb::setup(spawner, driver, config_store).await;

    info!("set up i2c ");
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, Config::default());

    info!("Setting up display");
    let mut matrix = CharlieWing::configure(i2c);
    unwrap!(matrix.setup(&mut Delay {}).await, "Failed to setup display");
    let mut rng = Rand32::new(0);

    let mut default_expression = comms.default_expression.dyn_receiver().unwrap();
    let mut blink_expression = comms.blink_expression.dyn_receiver().unwrap();

    set_face(&mut matrix, default_expression.get().await.pixels).await;

    loop {
        let blink_wait = rng.rand_range(2000..10000);
        Timer::after_millis(blink_wait.into()).await;
        set_face(&mut matrix, blink_expression.get().await.pixels).await;
        Timer::after_millis(25).await;
        set_face(&mut matrix, default_expression.get().await.pixels).await;
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
