/*! `tcn75a` is an [`embedded_hal`](https://github.com/rust-embedded/embedded-hal) crate for
accessing [Microchip TCN75A](https://www.microchip.com/wwwproducts/TCN75A) temperature sensors
over an I2C bus.

The TCN75A consists of 4 registers and a writeable register pointer. Three registers are for
configuration, represented as the following:

* Sensor Configuration Register (various `enum`s)
* Temperature Hysteresis Register (`i16`, -2048 to 2047)
* Temperature Limit-Set Register (`i16`, -2048 to 2047)

The remaining register contains the current temperature as an `i16`, from -2048 to 2047.

To avoid redundant register reads and write, the `tcn75a` crate caches the contents of some
registers (particularly the register pointer and Sensor Configuration Register). At present,
the `tcn75a` crate therefore _only works on I2C buses with a single controller._ Multi-controller
operation is possible at the cost of performance, but not implemented. */
#![no_std]

use core::result::Result;
use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

mod config;
use config::*;

/** A struct for describing how to read and write a TCN75A temperature sensors' registers via an
[`embedded_hal`] implementation (for a single-controller I2C bus).

Internally, the struct caches information written to the temperature sensor to speed up future
reads and writes. Due to caching, this [Tcn75a] struct is only usable on I2C buses with a single
controller. */
pub struct Tcn75a<T>
where
    T: Read + Write + WriteRead,
{
    ctx: T,
    address: u8,
    reg: Option<u8>,
}

#[derive(Debug, PartialEq)]
/// Enum for describing possible error conditions when reading/writing a TCN75A temperature sensor.
pub enum Tcn75aError<R, W> {
    /** A temperature value was read successfully, but some bits were set that should always
    read as zero for the given resolution. */
    OutOfRange,
    /** The register pointer could not be set to write the desired register. Contains the error
    reason from [Write::Error]. */
    RegPtrError(W),
    /** Reading the desired register via `embedded_hal` failed. Contains a [Read::Error],
    propagated from the [`embedded_hal`] implementation. */
    ReadError(R),
    /** Writing the desired register via `embedded_hal` failed. Contains a [Write::Error],
    propagated from the [`embedded_hal`] implementation. */
    WriteError(W)
}

impl<T> Tcn75a<T>
where
    T: Read + Write + WriteRead,
{
    pub fn new(ctx: T, address: u8) -> Self {
        Tcn75a { ctx, address, reg: None }
    }

    pub fn set_reg_ptr(&mut self, ptr: u8) -> Result<(), Tcn75aError<<T as Read>::Error, <T as Write>::Error>> {
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
            Err(e) => {
                self.reg = None;
                Err(Tcn75aError::RegPtrError(e))
            }
        }
    }

    pub fn temperature(&mut self) -> Result<i16, Tcn75aError<<T as Read>::Error, <T as Write>::Error>> {
        let mut temp: [u8; 2] = [0u8; 2];

        self.set_reg_ptr(0x00)?;

        match self.ctx.try_read(self.address, &mut temp) {
            Ok(_) => {
                let temp_limited = i16::from_be_bytes(temp) >> 4;

                if temp_limited >= -2048 && temp_limited < 2048 {
                    Ok(temp_limited)
                } else {
                    Err(Tcn75aError::OutOfRange)
                }
            }
            Err(e) => Err(Tcn75aError::ReadError(e)),
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
    use std::io::ErrorKind;

    use super::{Tcn75a, Tcn75aError};
    use embedded_hal_mock::{MockError, i2c::{Mock as I2cMock, Transaction as I2cTransaction}};

    fn mk_tcn75a(expectations: &[I2cTransaction], addr: u8) -> Tcn75a<I2cMock> {
        let i2c = I2cMock::new(expectations);
        let tcn = Tcn75a::new(i2c, addr);

        tcn
    }

    #[test]
    fn set_reg_ptr() {
        let mut tcn = mk_tcn75a(&[
            I2cTransaction::write(0x48, vec![0]),
            I2cTransaction::write(0x48, vec![3])
        ], 0x48);

        assert_eq!(tcn.set_reg_ptr(0), Ok(()));
        assert_eq!(tcn.set_reg_ptr(3), Ok(()));
        assert_eq!(tcn.reg, Some(3));
    }

    #[test]
    #[should_panic(expected="Register pointer must be set to between 0 and 3 (inclusive).")]
    fn reg_ptr_out_of_bounds() {
        let mut tcn = mk_tcn75a(&[], 0x48);
        tcn.set_reg_ptr(4).unwrap();
    }

    #[test]
    fn set_reg_ptr_fail() {
        let mut tcn = mk_tcn75a(&[
            I2cTransaction::write(0x48, vec![0]),
            I2cTransaction::write(0x48, vec![1]).with_error(MockError::Io(ErrorKind::Other)),
            I2cTransaction::write(0x48, vec![1])
        ], 0x48);

        assert_eq!(tcn.set_reg_ptr(0), Ok(()));
        assert_eq!(tcn.reg, Some(0));
        assert_eq!(tcn.set_reg_ptr(1), Err(Tcn75aError::RegPtrError(MockError::Io(ErrorKind::Other))));
        assert_eq!(tcn.reg, None);
        assert_eq!(tcn.set_reg_ptr(1), Ok(()));
        assert_eq!(tcn.reg, Some(1));
    }

    #[test]
    #[should_panic(expected="i2c::write address mismatch")]
    fn wrong_addr() {
        let mut tcn = mk_tcn75a(&[
            I2cTransaction::write(0x47, vec![0]),
        ], 0x48);

        tcn.set_reg_ptr(0).unwrap();
    }
}
