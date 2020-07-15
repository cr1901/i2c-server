extern crate i2cdev;

use std::thread;
use std::time::{SystemTime, Duration};

use i2cdev::core::*;
#[cfg(unix)]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

const TEMP_SENSOR_ADDR: u16 = 0x48;

// real code should probably not use unwrap()
#[cfg(unix)]
fn i2cfun() -> Result<(), LinuxI2CError> {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-1", TEMP_SENSOR_ADDR)?;

    dev.smbus_write_byte_data(0x01, 0x60)?;

    loop {
        let raw = i16::from_be(dev.smbus_read_word_data(0x00).unwrap() as i16) >> 4;
        let cels : f32 = f32::from(raw) / 16.0;
        let fahr = 1.8*cels + 32.0;

        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        println!("{:?}, {:?}, {:?}, {:?}", now, raw, cels, fahr);
        thread::sleep(Duration::from_millis(1000));
    }
}

fn main() {
    #[cfg(unix)]
    i2cfun().unwrap();
}
