//! This example shows how to communicate asynchronous using i2c with external chips.
//!
//! Example written for the [`MCP23017 16-Bit I2C I/O Expander with Serial Interface`] chip.
//! (https://www.microchip.com/en-us/product/mcp23017)

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::block::ImageDef;
use embassy_rp::i2c::{self, Config};
use embassy_rp::pac::pwm::regs::ChCc;
use embassy_rp::peripherals::{I2C1, USB};
use embassy_rp::usb;
use embassy_time::{Delay, Timer};
use embedded_hal_async::i2c::I2c;
use is31fl3731_async::devices::CharlieWing;
use is31fl3731_async::IS31FL3731;
use oorandom::Rand32;
use {defmt_rtt as _, panic_probe as _};

mod webusb;

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

const FACE: [[u8; 15]; 7] = [
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x2f, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2f, 0x2f, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x2f, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x2f, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x2f, 0x2f, 0x00, 0x2f, 0x00, 0x2f, 0x00, 0x2f, 0x2f, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x2f, 0x2f, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
];

const BLINK_FACE: [[u8; 15]; 7] = [
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x2f, 0x2f, 0x2f, 0x2f, 0x00, 0x00, 0x00, 0x2f, 0x2f, 0x2f, 0x2f, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x2f, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x2f, 0x2f, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
];

#[embassy_executor::main]
async fn main_(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Create the driver, from the HAL.
    let driver = usb::Driver::new(p.USB, Irqs);
    webusb::setup(spawner, driver);

    let sda = p.PIN_2;
    let scl = p.PIN_3;

    info!("set up i2c ");
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, Config::default());

    info!("Setting up display");
    let mut matrix = CharlieWing::configure(i2c);
    unwrap!(matrix.setup(&mut Delay {}).await, "Failed to setup display");
    let mut rng = Rand32::new(0);

    set_face(&mut matrix, FACE).await;
    // for y in 0..7 {
    //     for x in 0..15 {
    //         unwrap!(
    //             matrix.pixel(x, y, FACE[y as usize][x as usize]).await,
    //             "Failed to set pixel light on"
    //         );
    //     }
    // }

    loop {
        Timer::after_millis(2500).await;
        set_face(&mut matrix, BLINK_FACE).await;
        Timer::after_millis(100).await;
        set_face(&mut matrix, FACE).await;
    }
}

async fn set_face<I2C, I2cError>(matrix: &mut IS31FL3731<I2C>, face: [[u8; 15]; 7])
where
    I2C: I2c<Error = I2cError>,
{
    for y in 0..7 {
        for x in 0..15 {
            unwrap!(
                matrix.pixel(x, y, face[y as usize][x as usize]).await,
                "Failed to set pixel light on"
            );
        }
    }
}