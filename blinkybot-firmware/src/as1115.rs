use defmt::{info, unwrap};
use embedded_hal_async::i2c::I2c;

const PETAL_MATIX_I2C_ADDR: u8 = 0x00;

pub struct As1115 {
    data: [u8; 8],
}

impl As1115 {
    pub fn new() -> Self {
        Self { data: [0u8; 8] }
    }

    pub fn clear(&mut self) {
        for data in self.data.iter_mut() {
            *data = 0x0;
        }
    }
    pub fn set_arm(&mut self, arm: usize, pixel: u8, value: bool) {
        if arm > 7 {
            return;
        }
        if pixel > 6 {
            return;
        }

        if value {
            self.data[arm] |= 1 << pixel;
        } else {
            self.data[arm] &= !(1 << pixel);
        }
    }

    pub fn set_rgb(&mut self, red: bool, green: bool, blue: bool) {
        for (index, value) in [red, green, blue].iter().enumerate() {
            if *value {
                self.data[index] |= 1 << 7;
            } else {
                self.data[index] &= !(1 << 7);
            }
        }
    }

    pub async fn init<I2C, I2cError>(&self, i2c: &mut I2C)
    where
        I2C: I2c<Error = I2cError>,
        I2cError: defmt::Format,
    {
        unwrap!(i2c.write(PETAL_MATIX_I2C_ADDR, &[0x9, 0x0]).await);
        unwrap!(i2c.write(PETAL_MATIX_I2C_ADDR, &[0xa, 0x0f]).await);
        unwrap!(i2c.write(PETAL_MATIX_I2C_ADDR, &[0xc, 0x81]).await);
        unwrap!(i2c.write(PETAL_MATIX_I2C_ADDR, &[0xb, 0x7]).await);
    }
    pub async fn sync<I2C, I2cError>(&self, i2c: &mut I2C)
    where
        I2C: I2c<Error = I2cError>,
        I2cError: defmt::Format,
    {
        for (index, val) in self.data.iter().enumerate() {
            //info!("{:x} {:x}", index, val);
            unwrap!(
                i2c.write(PETAL_MATIX_I2C_ADDR, &[index as u8 + 1, *val])
                    .await
            );
        }
    }
}
