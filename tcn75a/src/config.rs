use modular_bitfield::prelude::*;

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

/** One-Shot bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf */
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum OneShot {
    Disabled = 0,
    Enabled,
}

/** ADC Resolution bits in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf */
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum Resolution {
    Bits9 = 0,
    Bits10,
    Bits11,
    Bits12,
}

/** Fault Queue bits in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf */
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum FaultQueue {
    One = 0,
    Two,
    Four,
    Six,
}

/** Alert Polarity bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf */
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum AlertPolarity {
    ActiveLow = 0,
    ActiveHigh,
}

/** Comp/Int bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf */
#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum CompInt {
    Comparator = 0,
    Interrupt,
}

/** Shutdown bit in the Sensor Configuration Register.

Consult the TCN75A [datasheet] for information on the meanings of each variant.
Variant names will be similar to the datasheet (changes in the datasheet names
in subsequent silicon revisions may constitute a breaking API change).

[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf */
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
