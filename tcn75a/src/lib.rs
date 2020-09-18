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

    pub fn set_reg_ptr(&mut self, ptr: u8) -> Result<(), ()> {
        todo!()
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
