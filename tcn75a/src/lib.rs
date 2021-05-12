/*! `tcn75a` is an [Embedded HAL] crate for accessing [Microchip TCN75A][TCN75A] temperature
sensors over an I2C bus.

The TCN75A consists of 4 registers and a writeable register pointer. Three registers are for
configuration, represented as the following:

* Sensor Configuration Register (various [`enum`s][`ConfigReg`])
* Temperature Hysteresis Register ([`FixedI16::<U8>`] ([`I8F8`]), -128.0 to 127.5, 0.5 degrees
  Celsius resolution)
* Temperature Limit-Set Register ([`FixedI16::<U8>`] ([`I8F8`]), -128.0 to 127.5, 0.5 degrees
  Celsius resolution)

The remaining register contains the current temperature as an [`FixedI16::<U8>`] ([`I8F8`]),
from -128.0 to 127.9375 (variable increments based on [`Resolution`]).

To avoid redundant register reads and write, the `tcn75a` crate caches the contents of some
registers (particularly the register pointer and Sensor Configuration Register). At present,
the `tcn75a` crate therefore _only works on I2C buses with a single controller._ Multi-controller
operation is possible at the cost of performance, but not implemented.

[Embedded HAL]: https://github.com/rust-embedded/embedded-hal
[TCN75A]: https://www.microchip.com/wwwproducts/TCN75A
[`ConfigReg`]: ./struct.ConfigReg.html
[`FixedI16::<U8>`]: ../fixed/struct.FixedI16.html
[`I8F8`]: ../fixed/types/type.I8F8.html
[`Resolution`]: ./enum.Resolution.html
*/
#![no_std]

use core::convert::TryFrom;
use core::fmt;
use core::result::Result;
use embedded_hal::blocking::i2c::{Read, Write};
use fixed::types::I8F8;

mod config;
pub use config::*;

mod limit;
pub use limit::*;

mod temp;
pub use temp::*;

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

impl<T> fmt::Debug for Tcn75a<T>
where
    T: Read + Write,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Tcn75a")
            .field("ctx", &"HAL context")
            .field("address", &self.address)
            .field("reg", &self.reg)
            .field("cfg", &self.cfg)
            .finish()
    }
}

/// Enum for describing possible error conditions when reading/writing a TCN75A temperature sensor.
pub enum Tcn75aError<R, W>
where
    R: Read,
    W: Write,
{
    /** A temperature value was read successfully, but some bits were set that should always
    read as zero. This _may_ indicate that you are not reading a TCN75A.  */
    OutOfRange,
    /** The temperature limit registers were read successfully, but the values read were invalid
    (violate the [invariants]). Contains a [`LimitError`] describing why the values are invalid,
    and a tuple of `(I8F8, I8F8)`, representing the values which were read; the Hysteresis (Low)
    value is the left element, and the Limit-Set (High) is the right element.

    [invariants]: ./struct.Limits.html#invariants
    [`LimitError`]: ./enum.LimitError.html
    */
    LimitError {
        reason: LimitError,
        values: (I8F8, I8F8),
    },
    /** The register pointer could not be set to _read_ the desired register. Contains the error
    reason from [`Write::Error`]. For register writes, [`WriteError`] is returned if the register
    pointer failed to update.

    [`Write::Error`]: ../embedded_hal/blocking/i2c/trait.Write.html#associatedtype.Error
    [`WriteError`]: ./enum.Tcn75aError.html#variant.WriteError
    */
    RegPtrError(<W as Write>::Error),
    /** Reading the desired register via [`embedded_hal`] failed. Contains a [`Read::Error`],
    propagated from the [`embedded_hal`] implementation.

    [`Read::Error`]: ../embedded_hal/blocking/i2c/trait.Read.html#associatedtype.Error
    [`embedded_hal`]: ../embedded_hal/index.html
    */
    ReadError(<R as Read>::Error),
    /** Writing the desired register via [`embedded_hal`] failed. Contains a [`Write::Error`],
    propagated from the [`embedded_hal`] implementation.

    [`Write::Error`]: ../embedded_hal/blocking/i2c/trait.Write.html#associatedtype.Error
    [`embedded_hal`]: ../embedded_hal/index.html
    */
    WriteError(<W as Write>::Error),
}

impl<R, W> fmt::Display for Tcn75aError<R, W>
where
    R: Read,
    W: Write,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Tcn75aError::<R, W>::OutOfRange => write!(f, "temperature reading out of range"),
            Tcn75aError::<R, W>::LimitError { reason: _r, values } => write!(
                f,
                "limit registers out of range (lo: {}, hi: {})",
                values.0, values.1
            ),
            Tcn75aError::<R, W>::RegPtrError(_w) => write!(f, "error writing register pointer"),
            Tcn75aError::<R, W>::ReadError(_r) => write!(f, "generic read error"),
            Tcn75aError::<R, W>::WriteError(_w) => write!(f, "generic write error"),
        }
    }
}

impl<R, W> fmt::Debug for Tcn75aError<R, W>
where
    R: Read,
    W: Write,
    <R as Read>::Error: fmt::Debug,
    <W as Write>::Error: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Tcn75aError::<R, W>::OutOfRange => write!(fmt, "OutOfRange"),
            Tcn75aError::<R, W>::LimitError { reason, values } => fmt
                .debug_struct("LimitError")
                .field("reason", reason)
                .field("values", values)
                .finish(),
            Tcn75aError::<R, W>::RegPtrError(w) => fmt.debug_tuple("RegPtrError").field(w).finish(),
            Tcn75aError::<R, W>::ReadError(r) => fmt.debug_tuple("ReadError").field(r).finish(),
            Tcn75aError::<R, W>::WriteError(w) => fmt.debug_tuple("WriteError").field(w).finish(),
        }
    }
}

// Mainly for tests.
impl<R, W> PartialEq<Self> for Tcn75aError<R, W>
where
    R: Read,
    W: Write,
    <R as Read>::Error: PartialEq<<R as Read>::Error>,
    <W as Write>::Error: PartialEq<<W as Write>::Error>,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Tcn75aError::<R, W>::OutOfRange, Tcn75aError::<R, W>::OutOfRange) => true,
            (
                Tcn75aError::<R, W>::LimitError {
                    reason: sr,
                    values: sv,
                },
                Tcn75aError::<R, W>::LimitError {
                    reason: or,
                    values: ov,
                },
            ) => sr == or && sv == ov,
            (Tcn75aError::<R, W>::RegPtrError(s), Tcn75aError::<R, W>::RegPtrError(o)) => s == o,
            (Tcn75aError::<R, W>::ReadError(s), Tcn75aError::<R, W>::ReadError(o)) => s == o,
            (Tcn75aError::<R, W>::WriteError(s), Tcn75aError::<R, W>::WriteError(o)) => s == o,
            _ => false,
        }
    }
}

impl<R, W> Eq for Tcn75aError<R, W>
where
    R: Read,
    W: Write,
    Tcn75aError<R, W>: PartialEq<Self>,
{
}

impl<R, W> Clone for Tcn75aError<R, W>
where
    R: Read,
    W: Write,
    <R as Read>::Error: Clone,
    <W as Write>::Error: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Tcn75aError::<R, W>::OutOfRange => Tcn75aError::<R, W>::OutOfRange,
            Tcn75aError::<R, W>::LimitError { reason, values } => Tcn75aError::<R, W>::LimitError {
                reason: *reason,
                values: *values,
            },
            Tcn75aError::<R, W>::RegPtrError(w) => Tcn75aError::<R, W>::RegPtrError(w.clone()),
            Tcn75aError::<R, W>::ReadError(r) => Tcn75aError::<R, W>::ReadError(r.clone()),
            Tcn75aError::<R, W>::WriteError(w) => Tcn75aError::<R, W>::WriteError(w.clone()),
        }
    }
}

impl<R, W> Copy for Tcn75aError<R, W>
where
    R: Read,
    W: Write,
    <R as Read>::Error: Copy,
    <W as Write>::Error: Copy,
{
}

/** Convenience type for representing [`Tcn75aError`]s where `T` implements both [`Read`]
and [`Write`].

[`Tcn75aError`]: ./enum.Tcn75aError.html
[`Read`]: ../embedded_hal/blocking/i2c/trait.Read.html
[`Write`]: ../embedded_hal/blocking/i2c/trait.Write.html
*/
pub type Error<T> = Tcn75aError<T, T>;

impl<T> Tcn75a<T>
where
    T: Read + Write,
{
    /** Initializes all the data required to read and write a TCN75A on an I2C bus.

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

    /** Sets the internal TCN75A register pointer to the specified address.

    All functions of [`Tcn75a`] that read or write registers will automatically set the register
    pointer beforehand. The previous register pointer value set is cached. It may be useful to
    manually set the register yourself some time _before_ you need to perform repeated _reads_
    from the pointed-to register.

    # Arguments

    * `ptr`: Value to which to set the internal TCN75A register pointer.

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::I2cdev;
    # use embedded_hal::blocking::i2c::{Read, Write};
    # use fixed::types::I8F8;
    # use tcn75a::{Tcn75a, Tcn75aError};
    # fn main() -> Result<(), Tcn75aError<I2cdev, I2cdev>> {
    # let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    // All subsequent examples should assume tcn is a `Tcn75a`
    // struct created previously.
    // Set the register pointer ahead of time.
    tcn.set_reg_ptr(0)?;
    for _ in 0..10 {
        // Then read temp values as fast as possible.
        println!("Temperature is: {}", I8F8::from(tcn.temperature()?));
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
            .map(|_| {
                self.reg = Some(ptr);
            })
            .map_err(|e| {
                self.reg = None;
                Tcn75aError::RegPtrError(e)
            })
    }

    /* Future API? The value returned is an `i16` holding either a 9-bit, 10-bit, 11-bit, or
    12-bit temperature value that represents a `Q8.1` (0.5 degree resolution), `Q8.2` (0.25),
    `Q8.3` (0.125), or `Q8.4` (0.0625) fixed-point number in [`Q` format]. */

    /** Gets a raw (9-12 bit) temperature reading from the TCN75A.

    Returns the temperature using:

    * An I2C write transaction to set the register pointer (if necessary), and
    * An I2C read transaction to read the Ambient Temperature Register.

    For any `Ok` or `Err` return variant besides [`Tcn75aError::RegPtrError`], the register
    pointer cache will point to register 0 after this function returns. The sensor config
    cache is untouched.

    # Internals

    Currently the [`temperature`] function does not use the [`Resolution`] data in the config
    cache. Each measurement returned is treated as a [`Q8.8`][`Q` format] number with the
    least-significant 4 bits unused, and some of bits 4 to 7 _possibly_ unused.
    For instance, the same temperature reading at different resolutions might be as follows
    (plus a negative temperature, for comparison):

    <table>
        <thead>
            <tr><th><a href="./enum.Resolution.html"><code>Resolution</code></a></th><th>Temp (C)</th><th>Bit Representation</th></tr>
        </thead>
        </tbody>
            <tr>
                <td><a href="enum.Resolution.html#variant.Bits9"><code>Bits9</code></a></td>
                <td>30.5</td>
                <td><code>0b01001000_1000_0000</code> (0x4880)</td>
            </tr>
            <tr>
                <td><a href="enum.Resolution.html#variant.Bits10"><code>Bits10</code></a>
                </td><td>30.5 </td>
                <td><code>0b01001000_1000_0000</code> (0x4880)</td>
            </tr>
            <tr>
                <td><a href="enum.Resolution.html#variant.Bits11"><code>Bits11</code></a>
                </td><td>30.375</td>
                <td><code>0b01001000_0110_0000</code> (0x4860)</td>
            </tr>
            <tr>
                <td><a href="enum.Resolution.html#variant.Bits12"><code>Bits12</code></a>
                </td><td>30.4375</td>
                <td><code>0b01001000_0111_0000</code> (0x4870)</td>
            </tr>
            <tr>
                <td><a href="enum.Resolution.html#variant.Bits9"><code>Bits9</code></a></td>
                <td>-10.5</td>
                <td><code>0b11110101_1000_0000</code> (0xF580)</td>
            </tr>
        </tbody>
    </table>

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::I2cdev;
    # use embedded_hal::blocking::i2c::{Read, Write};
    # use fixed::types::I8F8;
    # use tcn75a::{Tcn75a, Tcn75aError, ConfigReg, Resolution};
    # fn main() -> Result<(), Tcn75aError<I2cdev, I2cdev>> {
    # let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    // Assume `tcn` and the controller were _just_ powered on.
    // 9-bit resolution (0.5 degrees).
    let temp = tcn.temperature()?;
    println!("Temperature is {:.1} degrees Celsius", f32::from(I8F8::from(temp)));
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
    * [`Tcn75aError::ReadError`]: Returned if the I2C read to get the temperature register
      contents failed.
    * [`Tcn75aError::OutOfRange`]: The I2C read succeeded, but some bits which _must_ be 0
      _regardless_ of resolution were 1.

      Currently an [`OutOfRange`][`Tcn75aError::OutOfRange`] error is conservative, because
      [`temperature`] does not use cached [`Resolution`] data; it will not detect e.g. "bits set
      that indicate a 12-bit value, but the [`Resolution`] is [`Resolution::Bits9`]".

    [`Tcn75aError::RegPtrError`]: ./enum.Tcn75aError.html#variant.RegPtrError
    [`Q` format]: https://en.wikipedia.org/wiki/Q_(number_format)
    [`temperature`]: ./struct.Tcn75a.html#method.temperature
    [`Resolution`]: ./enum.Resolution.html
    [`Tcn75aError::ReadError`]: ./enum.Tcn75aError.html#variant.ReadError
    [`Tcn75aError::OutOfRange`]: ./enum.Tcn75aError.html#variant.OutOfRange
    [`Resolution::Bits9`]: ./enum.Resolution.html#variant.Bits9
    */
    pub fn temperature(&mut self) -> Result<Temperature, Error<T>> {
        let mut temp: [u8; 2] = [0u8; 2];

        self.set_reg_ptr(0x00)?;
        self.ctx
            .read(self.address, &mut temp)
            .map_err(Tcn75aError::ReadError)?;

        let raw_temp = i16::from_be_bytes(temp);

        // TODO: Vary the number of its checked based on Resolution and cache
        // contents. Fall back to most conservative (9Bits) if unknown
        // Resolution.
        if (raw_temp & 0x000f) == 0 {
            Ok(Temperature(I8F8::from_bits(raw_temp)))
        } else {
            Err(Tcn75aError::OutOfRange)
        }
    }

    /** Gets the current configuration of the TCN75A.

    The contents of the Sensor Configuration Register are returned using:

    * An I2C write transaction to set the register pointer (if necessary), and
    * An I2C read transaction to read the Sensor Configuration Register (if necessary).

    The contents of the Sensor Configuration Register are cached; no I2C transaction occurs
    if the config cache contains a previously-read value.

    For an `Ok` variant return value, the cache behavior varies:

    * If the config cache is valid, neither the register pointer or the sensor config cache
      are touched by this function.
    * If the config cache is not valid, an `Ok` return value means the register cache points
      to register 1, and the sensor config cache is updated with the [`ConfigReg`] value read
      from the bus (the same value wrapped by `Ok`).

    For other cache behavior, see [`Errors`].

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::I2cdev;
    # use embedded_hal::blocking::i2c::{Read, Write};
    # use tcn75a::{Tcn75a, Tcn75aError, ConfigReg, Resolution, FaultQueue};
    # fn main() -> Result<(), Tcn75aError<I2cdev, I2cdev>> {
    # let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    let mut cfg = tcn.config_reg()?; // Let's change some settings!
    // Get higher resolution samples at the cost of longer time to sample.
    cfg.set_resolution(Resolution::Bits12);
    // 6 conversion cycles before asserting alert.
    cfg.set_fault_queue(FaultQueue::Six);
    tcn.set_config_reg(cfg)?; // This will only modify resolution and fault queue.
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
      The register pointer cache is flushed. The config register cache is untouched.
    * [`Tcn75aError::ReadError`]: Returned if the I2C read to get the config register
      contents failed. The register pointer cache is set to register 1. The config register
      cache is flushed.

    [`ConfigReg`]: ./struct.ConfigReg.html
    [`Errors`]: ./struct.Tcn75a.html#errors-2
    [`Tcn75aError::RegPtrError`]: ./enum.Tcn75aError.html#variant.RegPtrError
    [`Tcn75aError::ReadError`]: ./enum.Tcn75aError.html#variant.ReadError
    */
    pub fn config_reg(&mut self) -> Result<ConfigReg, Error<T>> {
        let mut buf: [u8; 1] = [0u8; 1];

        if let Some(curr) = self.cfg {
            return Ok(curr);
        }

        self.set_reg_ptr(0x01)?;
        let cfg = self
            .ctx
            .read(self.address, &mut buf)
            .map(|_| {
                let cfg = ConfigReg::from_bytes(buf);

                self.cfg = Some(cfg);
                cfg
            })
            .map_err(|e| {
                self.cfg = None;
                Tcn75aError::ReadError(e)
            })?;

        Ok(cfg)
        // Ok(buf.try_into().unwrap())
        // Ok(&*buf.try_into().unwrap())
    }

    /** Sets the current configuration of the TCN75A.

    The contents of the Sensor Configuration Register are written using a single I2C
    write transaction, which sets the register pointer and writes the the Sensor Configuration
    Register.

    The contents of the Sensor Configuration Register are cached; no I2C transaction occurs
    if the config cache contains a previously-read value.

    For an `Ok` variant return value, the register pointer cache points to register 1, and
    the sensor config cache is updated to the written value. On `Err`, the caches are flushed.

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::I2cdev;
    # use embedded_hal::blocking::i2c::{Read, Write};
    # use tcn75a::{Tcn75a, Tcn75aError, ConfigReg, CompInt, Limits};
    # use fixed::types::I8F8;
    # use fixed_macro::fixed;
    # use std::convert::TryInto;
    # fn main() -> Result<(), Tcn75aError<I2cdev, I2cdev>> {
    # let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    let mut cfg = ConfigReg::new();
    let limits: Limits = (fixed!(25.0: I8F8), fixed!(30.0: I8F8)).try_into().unwrap();
    // Attached to a microcontroller, use Interrupt mode when temperature
    // exceeds/falls below limits (alert pin asserts when temp goes above 30C,
    // and then again when temp falls below 25C).
    cfg.set_comp_int(CompInt::Interrupt);
    tcn.set_config_reg(cfg)?;
    tcn.set_limits(limits)?;
    # Ok(())
    # }
    # } else {
    # fn main() {
    # }
    # }
    # }
    ```

    # Errors

    * [`Tcn75aError::WriteError`]: Returned if the I2C write to set the config register failed.
      The register pointer and sensor config caches are flushed.

    [`Tcn75aError::WriteError`]: ./enum.Tcn75aError.html#variant.WriteError
    */
    pub fn set_config_reg(&mut self, cfg: ConfigReg) -> Result<(), Error<T>> {
        let mut buf: [u8; 2] = [0u8; 2];

        // Reg ptr
        buf[0] = 0x01;
        buf[1] = cfg.into_bytes()[0];

        self.ctx
            .write(self.address, &buf)
            .map(|_| {
                self.cfg = Some(cfg);
            })
            .map_err(|e| {
                self.reg = None;
                self.cfg = None;
                Tcn75aError::WriteError(e)
            })?;
        self.reg = Some(0x01);

        Ok(())
    }

    /** Retrieves the lower and upper temperature limits before the TCN75A asserts an alarm.

    The contents of the Hysteresis and Limit-Set Registers are returned using _two_ of:

    * An I2C write transaction to set the register pointer (if necessary), and
    * An I2C read transaction to read each register (always occurs).

    For an `Ok` variant return value, the register pointer cache points to register 3. For
    an `Err` variant return value, the register pointer cache's value _should not be relied
    upon_. The sensor config cache is untouched by this function.

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::I2cdev;
    # use embedded_hal::blocking::i2c::{Read, Write};
    # use tcn75a::{Tcn75a, Tcn75aError, ConfigReg, AlertPolarity, Limits};
    # use std::convert::TryInto;
    # fn main() -> Result<(), Tcn75aError<I2cdev, I2cdev>> {
    # let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    let mut cfg = ConfigReg::new();
    // 9-bit fixed-point numbers- 25.5C to 30C
    let limits = tcn.limits();
    match limits {
        Ok(lim) => {
            # lim;
            // ... Safe to continue
        },
        Err(e) => {
            match e {
                Tcn75aError::LimitError {
                    reason,
                    values,
                } => {
                    # reason;
                    # values;
                    // ... Uh-oh! Use set_limits() to correct the value.
                },
                _ => {
                    # e;
                    // ... Handle other errors as appropriate.
                }
            }
        }
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

    * [`Tcn75aError::RegPtrError`]: Returned if the I2C write to set the register pointer for
      _either_ of the above registers failed. The register pointer cache is flushed.
    * [`Tcn75aError::ReadError`]: Returned if the I2C read to get _either_ of the above register
      contents failed. The register pointer cache is set to register is either 2 or 3.
    * [`Tcn75aError::LimitError`]: Both registers were read successfully, but violated invariants
      assumed by this library. The error reason and the values read are returned, as described
      [above]. The register pointer cache is set to 3.

    [`Tcn75aError::RegPtrError`]: ./enum.Tcn75aError.html#variant.RegPtrError
    [`Tcn75aError::ReadError`]: ./enum.Tcn75aError.html#variant.ReadError
    [`Tcn75aError::LimitError`]: ./enum.Tcn75aError.html#variant.LimitError
    [above]: ./enum.Tcn75aError.html#variant.LimitError
    */
    pub fn limits(&mut self) -> Result<Limits, Error<T>> {
        let mut buf: [u8; 2] = [0u8; 2];
        let mut lim: (I8F8, I8F8) = (0.into(), 0.into());

        self.set_reg_ptr(0x02)?;
        lim.0 = self
            .ctx
            .read(self.address, &mut buf)
            .map(|_| I8F8::from_be_bytes(buf))
            .map_err(Tcn75aError::ReadError)?;

        self.set_reg_ptr(0x03)?;
        lim.1 = self
            .ctx
            .read(self.address, &mut buf)
            .map(|_| I8F8::from_be_bytes(buf))
            .map_err(Tcn75aError::ReadError)?;

        TryFrom::try_from(lim).map_err(|r| Tcn75aError::LimitError {
            reason: r,
            values: lim,
        })
    }

    /** Sets _both_ the lower and upper temperature limits, outside of which the TCN75A asserts
    an alarm.

    The contents of the Hysteresis and Limit-Set Registers are written using two I2C write
    transactions (one for each). The contents of the Hysteresis and Limit-Set Registers
    are not cached.

    For an `Ok` variant return value, the register pointer cache points to register 3. For
    an `Err` variant return value, the register pointer cache is flushed. The sensor config
    cache is untouched by this function. An `Ok` return value means that the TCN75A has been
    programmed such that the Hysteresis Register value is less than the Limit-Set Register
    value.

    Although the TCN75A can tolerate a Hysteresis Register value which exceeds the Limit-Set
    Register value, for simplicity, this crate attempts to [disallow] it. _At present, a failed
    write to the Limit-Set Register via `set_limits` may result in a Hysteresis Register value
    which exceeds the Limit-Set Register value_.

    # Examples

    To create a low temperature alert, treat an asserted alert pin _of either [polarity]_
    as the operating-normally condition. When the temperature drops to below the value in
    Hysteresis Register, the alert pin will deassert, indicating the temperature is too low and
    the CPU should correct it. The alert pin will reassert when the temperature exceeds the value
    in the Limit-Set Register, which indicates the temperature is okay again.

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::I2cdev;
    # use embedded_hal::blocking::i2c::{Read, Write};
    # use tcn75a::{Tcn75a, Tcn75aError, ConfigReg, AlertPolarity, Limits};
    # use std::convert::TryInto;
    # use fixed::types::I8F8;
    # use fixed_macro::fixed;
    # fn main() -> Result<(), Tcn75aError<I2cdev, I2cdev>> {
    # let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    let mut cfg = ConfigReg::new();
    // 9-bit fixed-point numbers- 25.5C to 30C
    let limits: Limits = (fixed!(25.5: I8F8), fixed!(30.0: I8F8)).try_into().unwrap();
    // Asserted alert is default active-low at power-on reset.
    // Let's still treat active-high as the "everything's okay" condition.
    cfg.set_alert_polarity(AlertPolarity::ActiveHigh);
    tcn.set_config_reg(cfg)?;
    tcn.set_limits(limits)?;
    # Ok(())
    # }
    # } else {
    # fn main() {
    # }
    # }
    # }
    ```

    # Errors

    * [`Tcn75aError::WriteError`]: Returned if the I2C write to set _either_ the Hysteresis or
      Limit-Set register failed. The register pointer cache is flushed.

    [disallow]: ./struct.Limits.html
    [`Limits`]: ./struct.Limits.html
    [polarity]: ./enum.AlertPolarity.html
    [`Tcn75aError::WriteError`]: ./enum.Tcn75aError.html#variant.WriteError
    */
    pub fn set_limits(&mut self, limits: Limits) -> Result<(), Error<T>> {
        let mut buf: [u8; 3] = [0u8; 3];
        let (lower, upper): (I8F8, I8F8) = limits.into();

        // Reg ptr
        buf[0] = 0x02;
        buf[1..3].copy_from_slice(&lower.to_be_bytes());

        self.ctx.write(self.address, &buf).map_err(|e| {
            // TODO: PartialUpdate variant?
            self.reg = None;
            Tcn75aError::WriteError(e)
        })?;
        self.reg = Some(0x02); // Needed?

        // Reg ptr
        buf[0] = 0x03;
        buf[1..3].copy_from_slice(&upper.to_be_bytes());
        self.ctx.write(self.address, &buf).map_err(|e| {
            self.reg = None;
            Tcn75aError::WriteError(e)
        })?;
        self.reg = Some(0x03);

        Ok(())
    }

    /** Release the resources used to perform TCN75A transactions.

    No I2C transactions occur in this function. The wrapped [`embedded_hal`] instance is
    returned. You can call [`Tcn75a::new`] again with the returned instance to create a new
    `Tcn75a` struct associated with the same (or a different) TCN75A device with undefined
    caches.

    # Examples

    ```
    # cfg_if::cfg_if! {
    # if #[cfg(any(target_os = "linux", target_os = "android"))] {
    # use linux_embedded_hal::i2cdev::linux::LinuxI2CError;
    # fn main() -> Result<(), LinuxI2CError> {
    # use tcn75a::Tcn75a;
    # use linux_embedded_hal::I2cdev;
    # let i2c = I2cdev::new("/dev/i2c-1")?;
    # let mut tcn = Tcn75a::new(i2c, 0x48);
    let i2c = tcn.free(); // Get I2C HAL wrapper back.
    // ... Use the I2C wrapper to talk to other devices.
    let mut tcn = Tcn75a::new(i2c, 0x48); // Then reattach.
    // ... Reinitialize config registers, etc.
    # Ok::<(), LinuxI2CError>(())
    # }
    # } else {
    # fn main() {
    # }
    # }
    # }
    ```

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

    use super::{
        AlertPolarity, ConfigReg, LimitError, OneShot, Resolution, Shutdown, Tcn75a, Tcn75aError,
    };
    use embedded_hal_mock::{
        i2c::{Mock as I2cMock, Transaction as I2cTransaction},
        MockError,
    };
    use fixed::types::I8F8;
    use fixed_macro::fixed;

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

        // Compare against raw value, not corrected for 9-12 bits (divide by 16 in all cases to
        // get Celsius temp). In addition, we shift by 4 more bits to account for the 4 unused
        // bits.
        // (127 << 4) + 8 <= Q8.4
        // << 4 <= Q8.8/I8F8
        let temp = tcn.temperature();
        assert!(temp.is_ok());
        assert_eq!(
            I8F8::from(temp.unwrap()),
            I8F8::from_bits(((127 << 4) + 8) << 4)
        );
        let temp = tcn.temperature();
        assert!(temp.is_ok());
        assert_eq!(
            I8F8::from(temp.unwrap()),
            I8F8::from_bits(((127 << 4) + 8) << 4)
        );

        let i2c = tcn.free();
        let mut tcn = Tcn75a::new(i2c, 0x49);
        assert_eq!(tcn.address, 0x49);
        assert_eq!(tcn.reg, None);
        assert_eq!(tcn.cfg, None);

        let temp = tcn.temperature();
        assert!(temp.is_ok());
        assert_eq!(
            I8F8::from(temp.unwrap()),
            I8F8::from_bits(((0 << 4) - 1) << 4)
        );
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

        let temp = tcn.temperature();
        assert!(temp.is_err());
        assert_eq!(temp.unwrap_err(), Tcn75aError::OutOfRange);
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
                I2cTransaction::write(0x48, vec![0]).with_error(MockError::Io(ErrorKind::Other)),
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

        assert_eq!(
            tcn.set_config_reg(cfg1),
            Err(Tcn75aError::WriteError(MockError::Io(ErrorKind::Other)))
        );
        assert_eq!(tcn.cfg, None);
        assert_eq!(
            tcn.set_reg_ptr(0),
            Err(Tcn75aError::RegPtrError(MockError::Io(ErrorKind::Other)))
        );
        assert_eq!(
            tcn.config_reg(),
            Err(Tcn75aError::ReadError(MockError::Io(ErrorKind::Other)))
        );
        assert_eq!(tcn.config_reg(), Ok(cfg2));
        assert_eq!(tcn.set_config_reg(cfg1), Ok(()));
        assert_eq!(tcn.config_reg(), Ok(cfg1));
    }

    #[test]
    fn write_error_then_read() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![1, 0b10000101]),
                // Cache value reset on write error.
                I2cTransaction::write(0x48, vec![1, 0b01100000])
                    .with_error(MockError::Io(ErrorKind::Other)),
                I2cTransaction::write(0x48, vec![1]),
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

        assert_eq!(
            tcn.set_limits((fixed!(90.0: I8F8), fixed!(95.0: I8F8)).try_into().unwrap()),
            Ok(())
        );
        assert_eq!(
            tcn.limits().unwrap().try_into(),
            Ok((fixed!(90.0: I8F8), fixed!(95.0: I8F8)))
        );
    }

    #[test]
    fn read_limits_err() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![2]),
                I2cTransaction::read(0x48, vec![0x5a, 0xc0]),
                I2cTransaction::write(0x48, vec![3]),
                I2cTransaction::read(0x48, vec![0x5f, 0x00]),
            ],
            0x48,
        );

        assert_eq!(
            tcn.limits(),
            Err(Tcn75aError::LimitError {
                reason: LimitError::LowOutOfRange,
                values: (fixed!(90.75: I8F8), fixed!(95.0: I8F8))
            })
        );
    }

    #[test]
    fn write_limits_cache_partial_update() {
        let mut tcn = mk_tcn75a(
            &[
                I2cTransaction::write(0x48, vec![2, 0x5a, 0x00]),
                I2cTransaction::write(0x48, vec![3, 0x5f, 0x00])
                    .with_error(MockError::Io(ErrorKind::Other)),
                I2cTransaction::write(0x48, vec![2]),
                I2cTransaction::read(0x48, vec![0x5a, 0x00]),
                I2cTransaction::write(0x48, vec![3]),
                // Technically undefined value- don't actually care what the value is.
                // Use 0x5f/95 as a placeholder.
                I2cTransaction::read(0x48, vec![0x5f, 0x00]),
            ],
            0x48,
        );

        assert_eq!(
            tcn.set_limits((fixed!(90.0: I8F8), fixed!(95.0: I8F8)).try_into().unwrap()),
            Err(Tcn75aError::WriteError(MockError::Io(ErrorKind::Other)))
        );
        assert_eq!(
            tcn.limits().unwrap().try_into(),
            Ok((fixed!(90.0: I8F8), fixed!(95.0: I8F8)))
        );
    }
}
