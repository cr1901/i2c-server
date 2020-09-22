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
    reg: Option<u8>,
}

#[allow(dead_code)]
enum TempReadError<E> {
    OutOfRange,
    BusError(E),
}

impl<T> Tcn75a<T>
where
    T: Read + Write + WriteRead,
{
    pub fn new(ctx: T, address: u8) -> Self {
        Tcn75a { ctx, address, reg: None }
    }

    pub fn set_reg_ptr(&mut self, ptr: u8) -> Result<(), ()> {
        if ptr > 3 {
            panic!("Register pointer must be set to between 0 and 3 (inclusive).");
        }

        if let Some(curr) = self.reg {
            if curr == ptr {
                return Ok(())
            }
        }

        match self.ctx.try_write(self.address, &ptr.to_le_bytes()) {
            Ok(_) => {
                self.reg = Some(ptr);
                Ok(())
            },
            Err(_e) => {
                self.reg = None;
                Err(())
            }
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
            Err(_e) => Err(()),
        }
    }

    pub fn set_config_reg(&mut self, _reg: u8) {
        todo!()
    }

    pub fn update_config_reg<U>(&mut self, _reg: U)
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

    use super::{Tcn75a};
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
        assert_eq!(tcn.reg, Some(3));
    }

    #[test]
    #[should_panic(expected="Register pointer must be set to between 0 and 3 (inclusive).")]
    fn reg_ptr_out_of_bounds() {
        let expectations = [];

        let i2c = I2cMock::new(&expectations);
        let mut tcn = Tcn75a::new(i2c, 0x48);
        tcn.set_reg_ptr(4).unwrap();
    }
}
