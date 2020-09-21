#![no_std]

use core::result::Result;
use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

mod config;
use config::*;

pub struct Tcn75a<T>
where
    T: Read + Write + WriteRead,
{
    ctx: T,
    address: u8,
}

enum TempReadError<E> {
    OutOfRange,
    BusError(E),
}

impl<T> Tcn75a<T>
where
    T: Read + Write + WriteRead,
{
    pub fn new(ctx: T, address: u8) -> Self {
        Tcn75a { ctx, address }
    }

    fn set_reg_ptr(&mut self, ptr: u8) -> Result<(), ()> {
        match self.ctx.try_write(self.address, &ptr.to_le_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => Err(()),
        }
    }

    pub fn temperature(&mut self) -> Result<i16, ()> {
        let mut temp: [u8; 2] = [0u8; 2];

        self.set_reg_ptr(0x00)?;

        match self.ctx.try_read(self.address, &mut temp) {
            Ok(_) => {
                let temp_limited = i16::from_be_bytes(temp) >> 4;

                if temp_limited >= -2048 && temp_limited < 2048 {
                    Ok(temp_limited)
                } else {
                    Err(())
                }
            }
            Err(e) => Err(()),
        }
    }

    pub fn set_config_reg(&mut self, reg: u8) {
        todo!()
    }

    pub fn update_config_reg<U>(&mut self, reg: U)
    where
        U: Into<ConfigReg>,
    {
        todo!()
    }

    pub fn free(self) -> T {
        self.ctx
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec;

    use super::{Read, Write, WriteRead, Tcn75a};
    use embedded_hal_mock::i2c::{Mock as I2cMock, Transaction as I2cTransaction};

    #[test]
    fn test_reg_ptr() {
        let expectations = [
            I2cTransaction::write(0x48, vec![0]),
            I2cTransaction::write(0x48, vec![3])
        ];

        let i2c = I2cMock::new(&expectations);
        let mut tcn = Tcn75a::new(i2c, 0x48);

        assert_eq!(tcn.set_reg_ptr(0), Ok(()));
        assert_eq!(tcn.set_reg_ptr(3), Ok(()));
    }
}
