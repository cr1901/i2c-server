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
use core::convert::TryInto;
use core::result::Result;
use embedded_hal::blocking::i2c::{Read, Write};

mod config;
pub use config::*;

mod limit;
pub use limit::*;

/** A struct for describing how to read and write a TCN75A temperature sensors' registers via an
[`embedded_hal`] implementation (for a single-controller I2C bus).

Internally, the struct caches information written to the temperature sensor to speed up future
reads. Due to caching, this [`Tcn75a`] struct is only usable on I2C buses with a single
controller.

[`Tcn75a`]: ./struct.Tcn75a.html
[`embedded_hal`]: ../embedded_hal/index.html
*/
pub struct Tcn75a<T>
where
    T: Read + Write,
{
    ctx: T,
    address: u8,
    reg: Option<u8>,
    cfg: Option<ConfigReg>,
}

#[derive(Debug, PartialEq)]
/// Enum for describing possible error conditions when reading/writing a TCN75A temperature sensor.
pub enum Tcn75aError<R, W> {
    /** A temperature value was read successfully, but some bits were set that should always
    read as zero for the given resolution. */
    OutOfRange,
    /** The temperature limit registers were read successfully, but the values read were invalid
    (violate the [invariants]). Contains a [`LimitError`] describing why the values are invalid.

    [invariants]: ./struct.Limits.html#invariants
    [`LimitError`]: ./enum.LimitError.html
    */
    LimitError(LimitError),
    /** The register pointer could not be set to _read_ the desired register. Contains the error
    reason from [`Write::Error`]. For register writes, [`WriteError`] is returned if the register
    pointer failed to update.

    [`Write::Error`]: ../embedded_hal/blocking/i2c/trait.Write.html
    [`WriteError`]: ./enum.Tcn75aError.html#variant.WriteError
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
    T: Read + Write,
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
            cfg: None,
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

        let raw_temp = i16::from_be_bytes(temp);

        // TODO: Vary the number of its checked based on Resolution and cache
        // contents. Fall back to most conservative (9Bits) if unknown
        // Resolution.
        if (raw_temp & 0x000f) == 0 {
            Ok(raw_temp >> 4)
        } else {
            Err(Tcn75aError::OutOfRange)
        }
    }

    pub fn config_reg(&mut self) -> Result<ConfigReg, Error<T>> {
        let mut buf: [u8; 1] = [0u8; 1];

        if let Some(curr) = self.cfg {
            return Ok(curr);
        }

        self.set_reg_ptr(0x01)?;
        let cfg = self.ctx
            .read(self.address, &mut buf)
            .and_then(|_| {
                let buf_slice: &[u8] = &buf;
                let cfg = buf_slice.try_into().unwrap();

                self.cfg = Some(cfg);
                Ok(cfg)
            })
            .or_else(|e| {
                self.cfg = None;
                Err(Tcn75aError::ReadError(e))
            })?;

        Ok(cfg)
        // Ok(buf.try_into().unwrap())
        // Ok(&*buf.try_into().unwrap())
    }

    pub fn set_config_reg(&mut self, cfg: ConfigReg) -> Result<(), Error<T>> {
        let mut buf: [u8; 2] = [0u8; 2];

        // Reg ptr
        buf[0] = 0x01;
        buf[1] = cfg.to_bytes()[0];

        self.ctx
            .write(self.address, &buf)
            .and_then(|_| {
                self.cfg = Some(cfg);
                Ok(())
            })
            .or_else(|e| {
                self.cfg = None;
                Err(Tcn75aError::WriteError(e))
            })?;
        self.reg = Some(0x01);

        Ok(())
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
        let mut buf: [u8; 3] = [0u8; 3];
        let (mut lower, mut upper) = limits.into();

        // Reg ptr
        buf[0] = 0x02;
        lower <<= 7;
        &buf[1..3].copy_from_slice(&lower.to_be_bytes());

        self.ctx
            .write(self.address, &buf)
            .map_err(|e| Tcn75aError::WriteError(e))?;
        self.reg = Some(0x02);

        // Reg ptr
        buf[0] = 0x03;
        upper <<= 7;
        &buf[1..3].copy_from_slice(&upper.to_be_bytes());
        self.ctx
            .write(self.address, &buf)
            .map_err(|e| Tcn75aError::WriteError(e))?;
        self.reg = Some(0x03);

        Ok(())
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
    use std::convert::TryInto;
    use std::io::ErrorKind;
    use std::vec;

    use super::{Tcn75a, Tcn75aError, ConfigReg, Resolution, OneShot, AlertPolarity, Shutdown};
    use embedded_hal_mock::{
        i2c::{Mock as I2cMock, Transaction as I2cTransaction},
        MockError,
    };

    fn mk_tcn75a(expectations: &[I2cTransaction], addr: u8) -> Tcn75a<I2cMock> {
        let i2c = I2cMock::new(expectations);
        let tcn = Tcn75a::new(i2c, addr);

        tcn
    }

    fn mk_cfg_regs() -> (ConfigReg, ConfigReg) {
        let mut cfg1 = ConfigReg::new();
        cfg1.set_resolution(Resolution::Bits12);

        let mut cfg2 = ConfigReg::new();
        cfg2.set_one_shot(OneShot::Enabled);
        cfg2.set_alert_polarity(AlertPolarity::ActiveHigh);
        cfg2.set_shutdown(Shutdown::Enable);

        (cfg1, cfg2)
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
        // Already cached- no I2C write.
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

    #[test]
    fn create_read_free() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![0]),
                // Fake temp data
                I2cTransaction::read(0x48, vec![0x7f, 0x80]),
                // Cache initialized.
                I2cTransaction::read(0x48, vec![0x7f, 0x80]),
                // Negative value (different addr).
                I2cTransaction::write(0x49, vec![0]),
                I2cTransaction::read(0x49, vec![0xff, 0xf0]),
            ],
            0x48,
        );

        // We return raw value, not corrected for 9-12 bits (divide by 16 in all cases to
        // get Celsius temp). TODO: Possibly make newtype Temperature(i16) or
        // Temperature(i16, Resolution)?
        assert_eq!(tcn.temperature(), Ok(2040));
        assert_eq!(tcn.temperature(), Ok(2040));

        let i2c = tcn.free();
        let mut tcn = Tcn75a::new(i2c, 0x49);
        assert_eq!(tcn.address, 0x49);
        assert_eq!(tcn.reg, None);
        assert_eq!(tcn.cfg, None);

        assert_eq!(tcn.temperature(), Ok(-1));
    }

    #[test]
    fn read_invalid() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![0]),
                I2cTransaction::read(0x48, vec![0x80, 0x01]),
            ],
            0x48,
        );

        assert_eq!(tcn.temperature(), Err(Tcn75aError::OutOfRange));
    }

    #[test]
    fn write_read_config() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![1, 0b01100000]),
                I2cTransaction::read(0x48, vec![0b01100000]),
            ],
            0x48,
        );

        let (cfg1, _) = mk_cfg_regs();

        // Set the config register and read it back.
        assert_eq!(tcn.cfg, None);
        assert_eq!(tcn.set_config_reg(cfg1), Ok(()));
        assert_eq!(tcn.cfg, Some(cfg1));
        assert_eq!(tcn.config_reg(), Ok(cfg1));
        assert_eq!(tcn.cfg, Some(cfg1));
    }

    #[test]
    fn read_config_cached() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![1, 0b01100000]),
                // Fake reg set.
                I2cTransaction::write(0x48, vec![0]),
                // Cached value doesn't match.
                I2cTransaction::write(0x48, vec![1]),
                I2cTransaction::read(0x48, vec![0b01100000]),
                // Cache value matches.
                I2cTransaction::read(0x48, vec![0b01100000]),
            ],
            0x48,
        );

        // All cfg reg tests have the same initial write as write_read_config().
        let (cfg1, _) = mk_cfg_regs();
        tcn.set_config_reg(cfg1).unwrap();

        // Change reg ptr, then reread the config reg twice.
        assert_eq!(tcn.set_reg_ptr(0), Ok(()));
        assert_eq!(tcn.config_reg(), Ok(cfg1));
        assert_eq!(tcn.cfg, Some(cfg1));
        assert_eq!(tcn.config_reg(), Ok(cfg1));
        assert_eq!(tcn.cfg, Some(cfg1));
    }

    #[test]
    fn write_new_config_data() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![1, 0b01100000]),
                // Cache matches, but write transaction.
                I2cTransaction::write(0x48, vec![1, 0b10000101]),
                // TODO: Test as-if fake multi-controller I2C bus.
                // I2cTransaction::read(0x48, vec![0b00000000]),
            ],
            0x48,
        );

        let (cfg1, cfg2) = mk_cfg_regs();
        tcn.set_config_reg(cfg1).unwrap();

        // Write new data to config reg.
        assert_eq!(tcn.set_config_reg(cfg2), Ok(()));
        assert_eq!(tcn.cfg, Some(cfg2));
        // Read data changed from underneath us!
        // assert_eq!(tcn.config_reg(), Ok(cfg_new));
        // assert_eq!(tcn.cfg, Some(cfg_new));
    }


    #[test]
    fn write_read_error_cached() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![1, 0b10000101]),
                // Cache value reset on write error.
                I2cTransaction::write(0x48, vec![1, 0b01100000])
                    .with_error(MockError::Io(ErrorKind::Other)),
                // Dummy write to set reg pointer that dies with error.
                I2cTransaction::write(0x48, vec![0])
                    .with_error(MockError::Io(ErrorKind::Other)),
                // Read error w/ cache set should be impossible for now.
                I2cTransaction::write(0x48, vec![1]),
                I2cTransaction::read(0x48, vec![0b10000101])
                    .with_error(MockError::Io(ErrorKind::Other)),
                // Setting the register pointer cache didn't error, so should be skipped.
                I2cTransaction::read(0x48, vec![0b10000101]),
                I2cTransaction::write(0x48, vec![1, 0b01100000]),
                // Cache behavior back to normal- no read here.
            ],
            0x48,
        );

        let (cfg1, cfg2) = mk_cfg_regs();
        tcn.set_config_reg(cfg2).unwrap();

        assert_eq!(tcn.set_config_reg(cfg1),
            Err(Tcn75aError::WriteError(MockError::Io(ErrorKind::Other))));
        assert_eq!(tcn.cfg, None);
        assert_eq!(tcn.set_reg_ptr(0),
            Err(Tcn75aError::RegPtrError(MockError::Io(ErrorKind::Other))));
        assert_eq!(tcn.config_reg(),
            Err(Tcn75aError::ReadError(MockError::Io(ErrorKind::Other))));
        assert_eq!(tcn.config_reg(), Ok(cfg2));
        assert_eq!(tcn.set_config_reg(cfg1), Ok(()));
        assert_eq!(tcn.config_reg(), Ok(cfg1));
    }

    #[test]
    fn write_error_then_read() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![1, 0b10000101]),
                // Cache value reset on read error.
                I2cTransaction::write(0x48, vec![1, 0b01100000])
                    .with_error(MockError::Io(ErrorKind::Other)),
                I2cTransaction::read(0x48, vec![0b10000101]),
                I2cTransaction::write(0x48, vec![1, 0b01100000]),
                // Cache behavior back to normal.
                I2cTransaction::read(0x48, vec![0b01100000]),
            ],
            0x48,
        );

        let (cfg1, cfg2) = mk_cfg_regs();
        tcn.set_config_reg(cfg2).unwrap();
        tcn.set_config_reg(cfg1).unwrap_err();

        assert_eq!(tcn.config_reg(), Ok(cfg2));
        assert_eq!(tcn.set_config_reg(cfg1), Ok(()));
        assert_eq!(tcn.config_reg(), Ok(cfg1));
    }

    #[test]
    fn write_read_limits() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![2, 0x5a, 0x00]),
                I2cTransaction::write(0x48, vec![3, 0x5f, 0x00]),
                I2cTransaction::write(0x48, vec![2]),
                I2cTransaction::read(0x48, vec![0x5a, 0x00]),
                I2cTransaction::write(0x48, vec![3]),
                I2cTransaction::read(0x48, vec![0x5f, 0x00]),
            ],
            0x48,
        );

        assert_eq!(tcn.set_limits((90 * 2, 95 * 2).try_into().unwrap()), Ok(()));
        assert_eq!(tcn.limits().unwrap().try_into(), Ok((90 * 2, 95 * 2)));
    }

    #[test]
    fn write_limits_cache_partial_update() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![2, 0x5a, 0x00]),
                I2cTransaction::write(0x48, vec![3, 0x5f, 0x00])
                    .with_error(MockError::Io(ErrorKind::Other)),
                I2cTransaction::read(0x48, vec![0x5a, 0x00]),
                I2cTransaction::write(0x48, vec![3]),
                // Technically undefined value- don't actually care what the value is.
                // Use 0x5f/95 as a placeholder.
                I2cTransaction::read(0x48, vec![0x5f, 0x00]),
            ],
            0x48,
        );

        assert_eq!(
            tcn.set_limits((90 * 2, 95 * 2).try_into().unwrap()),
            Err(Tcn75aError::WriteError(MockError::Io(ErrorKind::Other)))
        );
        assert_eq!(tcn.limits().unwrap().try_into(), Ok((90 * 2, 95 * 2)));
    }
}
