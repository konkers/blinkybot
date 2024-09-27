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
use embassy_rp::i2c::{self, Config, InterruptHandler};
use embassy_rp::pac::pwm::regs::ChCc;
use embassy_rp::peripherals::I2C1;
use embassy_time::{Delay, Timer};
use embedded_hal_async::i2c::I2c;
use is31fl3731_async::devices::CharlieWing;
use oorandom::Rand32;
use {defmt_rtt as _, panic_probe as _};

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
    I2C1_IRQ => InterruptHandler<I2C1>;
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

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let sda = p.PIN_2;
    let scl = p.PIN_3;

    info!("set up i2c ");
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, Config::default());

    info!("Setting up display");
    let mut matrix = CharlieWing::configure(i2c);
    unwrap!(matrix.setup(&mut Delay {}).await, "Failed to setup display");
    let mut rng = Rand32::new(0);

    for y in 0..7 {
        for x in 0..15 {
            // let x: u8 = rng
            //     .rand_range(core::ops::Range { start: 0, end: 16 })
            //     .try_into()
            //     .unwrap();
            // let y: u8 = rng
            //     .rand_range(core::ops::Range { start: 0, end: 8 })
            //     .try_into()
            //     .unwrap();
            unwrap!(
                matrix.pixel(x, y, FACE[y as usize][x as usize]).await,
                "Failed to set pixel light on"
            );
        }
    }
    loop {
        info!("ping");
        Timer::after_millis(1000).await;
    }
}
