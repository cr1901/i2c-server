use core::convert::{From, TryFrom};

/** A struct representing the Hysteresis and Limit-Set registers of the TCN75A.

The Hysteresis and Limit-Set register provide the lower and upper bounds respectively of
the temperature range that the TCN75A should monitor. Temperatures outside this range will
assert the alert pin on the TCN75A. Temperatures are represented as 9-bit signed integers, which
corresponds to 0.5 degrees Celsius precision.

# Invariants

Successfully creating an instance of this struct provides the following runtime invariants:

* The Hysteresis and Limit-Set register values fit within a 9-bit signed integer
  (`i16`, -256 to 255).
* The Hysteresis limit value is less than the Limit-Set register.

As part of upholding these invariants:

* It is not currently possible to individually access either value of a [`Limits`] struct during
  its lifetime.
* The [`Tcn75a`] inherent implementation operates on both the Hysteresis and Limit-Set registers
  in API functions instead of providing access to each individual register.

# Examples

A [`Limits`] struct is created by invoking [`try_from`] on a `(i16, i16)` tuple, where the
Hysteresis (Low) value is on the left, and the Limit-Set (High) value is on the right. A
[`From<Limits>`][`From`] implementation on `(i16, i16)` consumes a [`Limits`] struct and gets back
the original values:

```
# use std::convert::TryInto;
# use tcn75a::Limits;
let orig = (0, 255);
let lims : Limits = orig.try_into().unwrap();
let restored = lims.into();
assert_eq!(orig, restored);
```

[`Limits`]: ./struct.Limits.html
[`Tcn75a`]: ./struct.Tcn75a.html
[`try_from`]: https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#tymethod.try_from
[`From`]: https://doc.rust-lang.org/nightly/core/convert/trait.From.html
*/
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Limits(i16, i16);

/** Reasons a conversion from `(i16, i16)` to [`Limits`] may fail.

Due to its runtime guarantees, a [`Limits`] struct can only be created by invoking [`try_from`]
on a `(i16, i16)` tuple. [`LimitError`] is the associated [`Error`] type in the
[`TryFrom<(i16, i16)>>`][`TryFrom`] implementation on [`Limits`], and it contains all the reasons
a conversion from `(i16, i16)` can fail.

[`Limits`]: ./struct.Limits.html
[`try_from`]: https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#tymethod.try_from
[`LimitError`]: ./enum.LimitError.html
[`Error`]: https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#associatedtype.Error
[`TryFrom`]: https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html
*/
#[derive(Debug, PartialEq)]
pub enum LimitError {
    /** _Both_ the Hysteresis and Limit-Set values provided do not fit in a 9-bit signed
    integer. */
    BothOutOfRange,
    /** The Hysteresis value provided does not fit in a 9-bit signed integer. */
    LowOutOfRange,
    /** The Limit-Set value provided does not fit in a 9-bit signed integer. */
    HighOutOfRange,
    /** The Hysteresis value is greater than _or equal to_ the Limit-Set value provided. */
    LowExceedsHigh
}

impl TryFrom<(i16, i16)> for Limits {
    type Error = LimitError;

    fn try_from(val: (i16, i16)) -> Result<Self, Self::Error> {
        if (val.0 < -256 || val.0 > 255) && (val.1 < -256 || val.1 > 255) {
            Err(LimitError::BothOutOfRange)
        } else if val.0 < -256 || val.0 > 255 {
            Err(LimitError::LowOutOfRange)
        } else if val.1 < -256 || val.1 > 255 {
            Err(LimitError::HighOutOfRange)
        } else if val.0 >= val.1 {
            Err(LimitError::LowExceedsHigh)
        } else {
            Ok(Limits(val.0, val.1))
        }
    }
}

impl From<Limits> for (i16, i16) {
    fn from(limits: Limits) -> (i16, i16) {
        (limits.0, limits.1)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_ok() {
        assert_eq!(TryFrom::try_from((0, 255)), Ok(Limits(0, 255)));
        assert_eq!(TryFrom::try_from((-256, 255)), Ok(Limits(-256, 255)));
        assert_eq!(TryFrom::try_from((-256, -1)), Ok(Limits(-256, -1)));
    }

    #[test]
    fn test_limit_err() {
        assert_eq!(<Limits as TryFrom<(i16, i16)>>::try_from((-257, 256)), Err(LimitError::BothOutOfRange));
        assert_eq!(<Limits as TryFrom<(i16, i16)>>::try_from((0, 256)), Err(LimitError::HighOutOfRange));
        assert_eq!(<Limits as TryFrom<(i16, i16)>>::try_from((-257, 0)), Err(LimitError::LowOutOfRange));
        assert_eq!(<Limits as TryFrom<(i16, i16)>>::try_from((1, -1)), Err(LimitError::LowExceedsHigh));
    }
}
