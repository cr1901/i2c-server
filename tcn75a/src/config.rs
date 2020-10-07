use modular_bitfield::prelude::*;
use core::convert::{From, TryFrom};

/** Representation of the Sensor Configuration Register.

The Sensor Configuration Register of the TCN75A is eight bits wide and consists of
6 separate fields. Fields are accessed using `get_*` and `set_*` methods provided by the
[`modular_bitfield`] crate. See the [datasheet] for information on field meanings.

# Examples

Each field has a power-of-two number of valid options. Therefore the `set_*` methods should never
panic:

```
# use tcn75a::{ConfigReg, Resolution};
let mut cfg = ConfigReg::new();
assert_eq!(cfg.get_resolution(), Resolution::Bits9);

cfg.set_resolution(Resolution::Bits12);
assert_eq!(cfg.get_resolution(), Resolution::Bits12);
```

Using `set_*_checked` methods and [`unwrap`ping][`unwrap`] the `Result` should also be zero-cost:

```
# use tcn75a::{ConfigReg, Resolution};
let mut cfg = ConfigReg::new();
assert_eq!(cfg.get_resolution(), Resolution::Bits9);

cfg.set_resolution_checked(Resolution::Bits12).unwrap();
assert_eq!(cfg.get_resolution(), Resolution::Bits12);
```

[`modular_bitfield`]: ../modular_bitfield/index.html
[`unwrap`]: https://doc.rust-lang.org/nightly/core/result/enum.Result.html#method.unwrap
[`ConfigReg`]: ./struct.ConfigReg.html
[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
*/
#[bitfield]
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct ConfigReg {
    #[bits = 1]
    shutdown: Shutdown,
    #[bits = 1]
    comp_int: CompInt,
    #[bits = 1]
    alert_polarity: AlertPolarity,
    #[bits = 2]
    fault_queue: FaultQueue,
    #[bits = 2]
    resolution: Resolution,
    #[bits = 1]
    one_shot: OneShot,
}

/** Error type due to failed conversions from u8 into Configuration Register fields.

This type cannot be created by the user. The main use of this type is to handle invalid
user-supplied config register values for the [`Resolution`] and [`FaultQueue`] Configuration
Registers fields.:

```
# use std::convert::Into;
# use std::convert::TryInto;
# use tcn75a::Resolution;
# use tcn75a::ConfigRegValueError;
fn main() -> Result<(), ConfigRegValueError> {
    let res: Resolution = 9.try_into()?; // Fake user-supplied input. Always succeeds.
    Ok(())
}
```

[`Resolution`]: ./enum.Resolution.html
[`FaultQueue`]: ./enum.FaultQueue.html
*/
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ConfigRegValueError(());

/** One-Shot bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
*/
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum OneShot {
    Disabled = 0,
    Enabled,
}

/** ADC Resolution bits in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

You can convert the `u8` values `9`, `10`, `11`, and `12` into a [`Resolution`] and
vice-versa using [`TryFrom<u8>`][`TryFrom`] and [`From<Resolution>`][`From`] respectively:

```
# use std::convert::Into;
# use std::convert::TryInto;
# use tcn75a::Resolution;
# use tcn75a::ConfigRegValueError;
let res: Resolution = 9u8.try_into().unwrap();
let res_as_int: u8 = Resolution::Bits10.into();
let try_res_fail: Result<Resolution, ConfigRegValueError> = 13u8.try_into();

assert_eq!(res, Resolution::Bits9);
assert_eq!(res_as_int, 10u8);
assert!(try_res_fail.is_err());
```

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
[`Resolution`]: ./enum.Resolution.html
[`TryFrom`]: https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html
[`From`]: https://doc.rust-lang.org/nightly/core/convert/trait.From.html
*/
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum Resolution {
    Bits9 = 0,
    Bits10,
    Bits11,
    Bits12,
}

impl From<Resolution> for u8 {
    fn from(res: Resolution) -> u8 {
        match res {
            Resolution::Bits9 => 9,
            Resolution::Bits10 => 10,
            Resolution::Bits11 => 11,
            Resolution::Bits12 => 12,
        }
    }
}

impl TryFrom<u8> for Resolution {
    type Error = ConfigRegValueError;

    fn try_from(value: u8) -> Result<Resolution, Self::Error> {
        match value {
            9 => Ok(Resolution::Bits9),
            10 => Ok(Resolution::Bits10),
            11 => Ok(Resolution::Bits11),
            12 => Ok(Resolution::Bits12),
            _ => Err(ConfigRegValueError(())),
        }
    }
}

/** Fault Queue bits in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

You can convert the `u8` values `1`, `2`, `4`, and `6` into a [`FaultQueue`] and
vice-versa using [`TryFrom<u8>`][`TryFrom`] and [`From<FaultQueue>`][`From`] respectively:

```
# use std::convert::Into;
# use std::convert::TryInto;
# use tcn75a::FaultQueue;
# use tcn75a::ConfigRegValueError;
let fq: FaultQueue = 1u8.try_into().unwrap();
let fq_as_int: u8 = FaultQueue::Two.into();
let try_fq_fail: Result<FaultQueue, ConfigRegValueError> = 8u8.try_into();

assert_eq!(fq, FaultQueue::One);
assert_eq!(fq_as_int, 2u8);
assert!(try_fq_fail.is_err());
```

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
[`FaultQueue`]: ./enum.FaultQueue.html
[`TryFrom`]: https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html
[`From`]: https://doc.rust-lang.org/nightly/core/convert/trait.From.html

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
*/
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum FaultQueue {
    One = 0,
    Two,
    Four,
    Six,
}

impl From<FaultQueue> for u8 {
    fn from(fq: FaultQueue) -> u8 {
        match fq {
            FaultQueue::One => 1,
            FaultQueue::Two => 2,
            FaultQueue::Four => 4,
            FaultQueue::Six => 6,
        }
    }
}

impl TryFrom<u8> for FaultQueue {
    type Error = ConfigRegValueError;

    fn try_from(value: u8) -> Result<FaultQueue, Self::Error> {
        match value {
            1 => Ok(FaultQueue::One),
            2 => Ok(FaultQueue::Two),
            4 => Ok(FaultQueue::Four),
            6 => Ok(FaultQueue::Six),
            _ => Err(ConfigRegValueError(())),
        }
    }
}

/** Alert Polarity bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
*/
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum AlertPolarity {
    ActiveLow = 0,
    ActiveHigh,
}

/** Comp/Int bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
*/
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum CompInt {
    Comparator = 0,
    Interrupt,
}

/** Shutdown bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf
*/
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum Shutdown {
    Disable = 0,
    Enable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::TryInto;
    use core::mem::size_of;

    #[test]
    fn test_size() {
        assert_eq!(size_of::<ConfigReg>(), 1);
    }

    #[test]
    fn test_two_fields() {
        let mut cfg: ConfigReg = Default::default();
        cfg.set_shutdown(Shutdown::Disable);
        cfg.set_comp_int(CompInt::Interrupt);

        let val = u8::from_le_bytes(cfg.to_bytes().try_into().unwrap());

        assert_eq!(val, 0b0000010);
    }

    #[test]
    fn test_2bit_val() {
        let mut cfg = ConfigReg::new();
        cfg.set_resolution(Resolution::Bits12);
        cfg.set_fault_queue(FaultQueue::Six);

        let val = u8::from_le_bytes(cfg.to_bytes().try_into().unwrap());
        assert_eq!(val, 0b01111000);
    }

    #[test]
    fn test_reset_defaults() {
        let cfg = ConfigReg::new();

        assert_eq!(cfg.get_shutdown(), Shutdown::Disable);
        assert_eq!(cfg.get_comp_int(), CompInt::Comparator);
        assert_eq!(cfg.get_alert_polarity(), AlertPolarity::ActiveLow);
        assert_eq!(cfg.get_resolution(), Resolution::Bits9);
        assert_eq!(cfg.get_fault_queue(), FaultQueue::One);
        assert_eq!(cfg.get_one_shot(), OneShot::Disabled);

        let val = u8::from_le_bytes(cfg.to_bytes().try_into().unwrap());
        assert_eq!(val, 0);
    }
}
