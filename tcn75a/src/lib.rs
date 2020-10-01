/*! `tcn75a` is an [Embedded HAL] crate for accessing [Microchip TCN75A][TCN75A] temperature
sensors over an I2C bus.

The TCN75A consists of 4 registers and a writeable register pointer. Three registers are for
configuration, represented as the following:

* Sensor Configuration Register (various `enum`s)
* Temperature Hysteresis Register (`i16`, -256 to 255)
* Temperature Limit-Set Register (`i16`, -256 to 255)

The remaining register contains the current temperature as an `i16`, from -2048 to 2047.

To avoid redundant register reads and write, the `tcn75a` crate caches the contents of some
registers (particularly the register pointer and Sensor Configuration Register). At present,
the `tcn75a` crate therefore _only works on I2C buses with a single controller._ Multi-controller
operation is possible at the cost of performance, but not implemented.

[Embedded HAL]: https://github.com/rust-embedded/embedded-hal
[TCN75A]: https://www.microchip.com/wwwproducts/TCN75A

*/
#![no_std]

use core::convert::TryFrom;
use core::result::Result;
use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

mod config;
pub use config::*;

mod limit;
pub use limit::*;

/** A struct for describing how to read and write a TCN75A temperature sensors' registers via an
[`embedded_hal`] implementation (for a single-controller I2C bus).

Internally, the struct caches information written to the temperature sensor to speed up future
reads and writes. Due to caching, this [`Tcn75a`] struct is only usable on I2C buses with a single
controller.

[`Tcn75a`]: ./struct.Tcn75a.html
[`embedded_hal`]: ../embedded_hal/index.html
*/
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
    LimitError(LimitError),
    /** The register pointer could not be set to write the desired register. Contains the error
    reason from [Write::Error].

    [`Write::Error`]: ../embedded_hal/blocking/i2c/trait.Write.html
    */
    RegPtrError(W),
    /** Reading the desired register via [`embedded_hal`] failed. Contains a [`Read::Error`],
    propagated from the [`embedded_hal`] implementation.

    [`Read::Error`]: ../embedded_hal/blocking/i2c/trait.Read.html#associatedtype.Error
    [`embedded_hal`]: ../embedded_hal/index.html
    */
    ReadError(R),
    /** Writing the desired register via [`embedded_hal`] failed. Contains a [`Write::Error`],
    propagated from the [`embedded_hal`] implementation.

    [`Write::Error`]: ../embedded_hal/blocking/i2c/trait.Write.html#associatedtype.Error
    [`embedded_hal`]: ../embedded_hal/index.html
    */
    WriteError(W),
}

/** Convenience type for representing [`Tcn75aError`]s where `T` implements both [`Read`]
and [`Write`].

[`Tcn75aError`]: ./enum.Tcn75aError.html
[`Read`]: ../embedded_hal/blocking/i2c/trait.Read.html
[`Write`]: ../embedded_hal/blocking/i2c/trait.Write.html
*/
pub type Error<T> = Tcn75aError<<T as Read>::Error, <T as Write>::Error>;

impl<T> Tcn75a<T>
where
    T: Read + Write + WriteRead,
{
    /** Initialize all the data required to read and write a TCN75A on an I2C bus.

    No I2C transactions occur in this function.

    # Arguments

    * `ctx`: A type `T` implementing the [I2C traits] of [`embedded_hal`].
    * `address`: I2C address of the TCN75A sensor.

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::i2cdev::linux::LinuxI2CError;
    # fn main() -> Result<(), LinuxI2CError> {
    use tcn75a::Tcn75a;
    use linux_embedded_hal::I2cdev;

    let i2c = I2cdev::new("/dev/i2c-1")?;
    let mut tcn = Tcn75a::new(i2c, 0x48);
    # Ok::<(), LinuxI2CError>(())
    # }
    # } else {
    # fn main() {
    # }
    # }
    # }
    ```

    [I2C traits]: ../embedded_hal/blocking/i2c/index.html#traits
    [`embedded_hal`]: ../embedded_hal
    */
    pub fn new(ctx: T, address: u8) -> Self {
        Tcn75a {
            ctx,
            address,
            reg: None,
        }
    }

    /** Set the internal TCN75A register pointer to the specified address.

    All functions of [`Tcn75a`] that read or write registers will automatically set the register
    pointer beforehand. The previous register pointer value set is cached. It may be useful to
    manually set the register yourself some time _before_ you need to perform a write or read to
    the pointed-to register.

    # Arguments

    * `ptr`: Value to which to set the internal TCN75A register pointer.

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::I2cdev;
    # use embedded_hal::blocking::i2c::{Read, Write};
    # use tcn75a::{Tcn75a, Tcn75aError};
    # fn main() -> Result<(), Tcn75aError<<I2cdev as Read>::Error, <I2cdev as Write>::Error>> {
    # let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    // Set the register pointer ahead of time.
    // Then read temp values as fast as possible.
    tcn.set_reg_ptr(0)?;
    for _ in 0..10 {
        println!("Temperature is: {}", tcn.temperature()?);
    }
    # Ok(())
    # }
    # } else {
    # fn main() {
    # }
    # }
    # }
    ```

    # Errors

    * [`Tcn75aError::RegPtrError`]: Returned if the I2C write to set the register pointer failed.
      The register pointer cache is flushed.

    # Panics

    This function panics if `ptr` is greater than 3; the TCN75A has 4 registers starting at offset
    0.

    [`Tcn75a`]: ./struct.Tcn75a.html
    [`Tcn75aError::RegPtrError`]: ./enum.Tcn75aError.html#variant.RegPtrError
    */
    pub fn set_reg_ptr(&mut self, ptr: u8) -> Result<(), Error<T>> {
        if ptr > 3 {
            panic!("Register pointer must be set to between 0 and 3 (inclusive).");
        }

        if let Some(curr) = self.reg {
            if curr == ptr {
                return Ok(());
            }
        }

        self.ctx
            .write(self.address, &ptr.to_le_bytes())
            .and_then(|_| {
                self.reg = Some(ptr);
                Ok(())
            })
            .or_else(|e| {
                self.reg = None;
                Err(Tcn75aError::RegPtrError(e))
            })
    }

    pub fn temperature(
        &mut self,
    ) -> Result<i16, Tcn75aError<<T as Read>::Error, <T as Write>::Error>> {
        let mut temp: [u8; 2] = [0u8; 2];

        self.set_reg_ptr(0x00)?;
        self.ctx
            .read(self.address, &mut temp)
            .map_err(|e| Tcn75aError::ReadError(e))?;

        let temp_limited = i16::from_be_bytes(temp) >> 4;

        if temp_limited >= -2048 && temp_limited < 2048 {
            Ok(temp_limited)
        } else {
            Err(Tcn75aError::OutOfRange)
        }
    }

    pub fn config_reg(&self) -> ConfigReg {
        todo!()
    }

    pub fn set_config_reg(&mut self, _reg: u8) {
        todo!()
    }

    pub fn limits(
        &mut self,
    ) -> Result<Limits, Tcn75aError<<T as Read>::Error, <T as Write>::Error>> {
        let mut buf: [u8; 2] = [0u8; 2];
        let mut lim = (0i16, 0i16);

        self.set_reg_ptr(0x02)?;
        lim.0 = self
            .ctx
            .read(self.address, &mut buf)
            .and_then(|_| Ok(i16::from_be_bytes(buf) >> 7))
            .map_err(|e| Tcn75aError::ReadError(e))?;

        self.set_reg_ptr(0x03)?;
        lim.1 = self
            .ctx
            .read(self.address, &mut buf)
            .and_then(|_| Ok(i16::from_be_bytes(buf) >> 7))
            .map_err(|e| Tcn75aError::ReadError(e))?;

        TryFrom::try_from(lim).map_err(|e| Tcn75aError::LimitError(e))
    }

    pub fn set_limits(
        &mut self,
        limits: Limits,
    ) -> Result<(), Tcn75aError<<T as Read>::Error, <T as Write>::Error>> {
        let (mut lower, mut upper) = limits.into();

        self.set_reg_ptr(0x02)?;
        lower <<= 7;
        self.ctx
            .write(self.address, &lower.to_be_bytes())
            .map_err(|e| Tcn75aError::WriteError(e))?;

        self.set_reg_ptr(0x03)?;
        upper <<= 7;
        self.ctx
            .write(self.address, &upper.to_be_bytes())
            .map_err(|e| Tcn75aError::WriteError(e))
    }

    /** Release the resources used to perform TCN75A transactions.

    No I2C transactions occur in this function. The wrapped [`embedded_hal`] instance is
    returned. You can call [`Tcn75a::new`] again with the returned instance to create a new
    `Tcn75a` struct associated with the same device (with undefined caches).

    [`embedded_hal`]: ../embedded_hal
    [`Tcn75a::new`]: ../struct.Tcn75a.html#method.new
    */
    pub fn free(self) -> T {
        self.ctx
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::io::ErrorKind;
    use std::vec;

    use super::{Tcn75a, Tcn75aError};
    use embedded_hal_mock::{
        i2c::{Mock as I2cMock, Transaction as I2cTransaction},
        MockError,
    };

    fn mk_tcn75a(expectations: &[I2cTransaction], addr: u8) -> Tcn75a<I2cMock> {
        let i2c = I2cMock::new(expectations);
        let tcn = Tcn75a::new(i2c, addr);

        tcn
    }

    #[test]
    fn set_reg_ptr() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![0]),
                I2cTransaction::write(0x48, vec![3]),
            ],
            0x48,
        );

        assert_eq!(tcn.set_reg_ptr(0), Ok(()));
        assert_eq!(tcn.set_reg_ptr(3), Ok(()));
        assert_eq!(tcn.reg, Some(3));
    }

    #[test]
    #[should_panic(expected = "Register pointer must be set to between 0 and 3 (inclusive).")]
    fn reg_ptr_out_of_bounds() {
        let mut tcn = mk_tcn75a(&[], 0x48);
        tcn.set_reg_ptr(4).unwrap();
    }

    #[test]
    fn set_reg_ptr_fail() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![0]),
                I2cTransaction::write(0x48, vec![1]).with_error(MockError::Io(ErrorKind::Other)),
                I2cTransaction::write(0x48, vec![1]),
            ],
            0x48,
        );

        assert_eq!(tcn.set_reg_ptr(0), Ok(()));
        assert_eq!(tcn.reg, Some(0));
        assert_eq!(
            tcn.set_reg_ptr(1),
            Err(Tcn75aError::RegPtrError(MockError::Io(ErrorKind::Other)))
        );
        assert_eq!(tcn.reg, None);
        assert_eq!(tcn.set_reg_ptr(1), Ok(()));
        assert_eq!(tcn.reg, Some(1));
    }

    #[test]
    #[should_panic(expected = "i2c::write address mismatch")]
    fn wrong_addr() {
        let mut tcn = mk_tcn75a(&[I2cTransaction::write(0x47, vec![0])], 0x48);

        tcn.set_reg_ptr(0).unwrap();
    }
}
