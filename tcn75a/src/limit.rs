use core::convert::{From, TryFrom};
use core::fmt;
use fixed::types::I8F8;

/** A struct representing the Hysteresis and Limit-Set registers of the TCN75A.

The Hysteresis and Limit-Set registers provide the lower and upper bounds respectively of
the temperature range that the TCN75A should monitor. When the temperature is above the value in
Limit-Set Register, the alert pin will assert. The alert pin will _remain_ asserted until the
temperature goes below the value in the Hysteresis register.

Temperatures are represented as 9-bit signed integers ([`Q8.1`]), which corresponds to 0.5 degrees
Celsius precision. Internally, this crate represents [`Q8.1`] integers as [`FixedI16::<U8>`]
types, or [`I8F8`] for short. To create values of type [`I8F8`], you are encouraged to use the
[`fixed_macro`] crate, as in the [example][limit_example].

The silicon can tolerate swapped Hysteresis and Limit-Set registers, but to avoid potential
confusing TCN75A behavior, this crate does not allow it in normal operation. For rationale as
to why the Hysteresis register can also be treated a low temperature alert, see the [`set_limits`]
documentation [examples].

# Invariants

Successfully creating an instance of this struct provides the following runtime invariants:

* The Hysteresis and Limit-Set register values fit within a [`Q8.1`] signed integer (either `0`
  or `0.5` is allowed as the fractional part).
* The Hysteresis register value is less than the Limit-Set register.

As part of upholding these invariants:

* It is not currently possible to individually access either value of a [`Limits`] struct during
  its lifetime.
* The [`Tcn75a`] inherent implementation operates on both the Hysteresis and Limit-Set registers
  in API functions instead of providing access to each individual register.

# Examples

A [`Limits`] struct is created by invoking [`try_from`] on a `(I8F8, I8F8)` tuple, where the
Hysteresis (Low) value is on the left, and the Limit-Set (High) value is on the right. A
[`From<Limits>`][`From`] implementation on `(I8F8, I8F8)` consumes a [`Limits`] struct and gets back
the original values:

```
# use std::convert::TryInto;
# use tcn75a::Limits;
use fixed::types::I8F8;
use fixed_macro::fixed;

let orig = (fixed!(0: I8F8), fixed!(127.5: I8F8));
let lims: Limits = orig.try_into().unwrap();
let restored = lims.into();
assert_eq!(orig, restored);
```

[`Q8.1`]: https://en.wikipedia.org/wiki/Q_(number_format)
[`FixedI16::<U8>`]: ../fixed/struct.FixedI16.html
[`I8F8`]: ../fixed/types/type.I8F8.html
[`fixed_macro`]: ../fixed_macro/index.html
[limit_example]: ./struct.Limits.html#examples
[`set_limits`]: ./struct.Tcn75a.html#method.set_limits
[examples]: ./struct.Tcn75a.html#examples-6
[`Limits`]: ./struct.Limits.html
[`Tcn75a`]: ./struct.Tcn75a.html
[`try_from`]: https://doc.rust-lang.org/nightly/core/convert/trait.TryFrom.html#tymethod.try_from
[`From`]: https://doc.rust-lang.org/nightly/core/convert/trait.From.html
*/
// TODO: Limits::new().
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Limits(I8F8, I8F8);

/** Reasons a conversion from `(I8F8, I8F8)` to [`Limits`] may fail.

Due to its runtime guarantees, a [`Limits`] struct can only be created by invoking [`try_from`]
on a `(I8F8, I8F8)` tuple. [`LimitError`] is the associated [`Error`] type in the
[`TryFrom<(I8F8, I8F8)>`][`TryFrom`] implementation on [`Limits`], and it contains all the reasons
a conversion from `(I8F8, I8F8)` can fail.

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
    LowExceedsHigh,
}

impl fmt::Display for LimitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LimitError::BothOutOfRange => write!(f, "both hysteresis and limit-set register values out of range"),
            LimitError::LowOutOfRange => write!(f, "hysteresis register value out of range"),
            LimitError::HighOutOfRange => write!(f, "limit-set register value out of range"),
            LimitError::LowExceedsHigh => write!(f, "hysteresis register value exceeds limit-set register value"),
        }
    }
}

impl TryFrom<(I8F8, I8F8)> for Limits {
    type Error = LimitError;

    fn try_from(val: (I8F8, I8F8)) -> Result<Self, Self::Error> {
        let half = I8F8::from_num(1) / 2;

        let lo_out_of_range = val.0.frac() != half && val.0.frac() != 0;
        let hi_out_of_range = val.1.frac() != half && val.1.frac() != 0;

        match (lo_out_of_range, hi_out_of_range) {
            (true, true) => Err(LimitError::BothOutOfRange),
            (true, false) => Err(LimitError::LowOutOfRange),
            (false, true) => Err(LimitError::HighOutOfRange),
            (false, false) => {
                if val.0 >= val.1 {
                    Err(LimitError::LowExceedsHigh)
                } else {
                    Ok(Limits(val.0, val.1))
                }
            }
        }
    }
}

impl From<Limits> for (I8F8, I8F8) {
    fn from(limits: Limits) -> (I8F8, I8F8) {
        (limits.0, limits.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixed_macro::fixed;

    #[test]
    fn test_limit_ok() {
        assert_eq!(
            TryFrom::try_from((fixed!(0: I8F8), fixed!(127.5: I8F8))),
            Ok(Limits(fixed!(0: I8F8), fixed!(127.5: I8F8)))
        );
        assert_eq!(
            TryFrom::try_from((fixed!(-128.0: I8F8), fixed!(127.5: I8F8))),
            Ok(Limits(fixed!(-128.0: I8F8), fixed!(127.5: I8F8)))
        );
        assert_eq!(
            TryFrom::try_from((fixed!(-128.0: I8F8), fixed!(-0.5: I8F8))),
            Ok(Limits(fixed!(-128.0: I8F8), fixed!(-0.5: I8F8)))
        );
    }

    #[test]
    fn test_limit_err() {
        assert_eq!(
            <Limits as TryFrom<(I8F8, I8F8)>>::try_from((
                fixed!(-127.75: I8F8),
                fixed!(127.75: I8F8)
            )),
            Err(LimitError::BothOutOfRange)
        );
        assert_eq!(
            <Limits as TryFrom<(I8F8, I8F8)>>::try_from((fixed!(0: I8F8), fixed!(127.75: I8F8))),
            Err(LimitError::HighOutOfRange)
        );
        assert_eq!(
            <Limits as TryFrom<(I8F8, I8F8)>>::try_from((fixed!(-127.75: I8F8), fixed!(0: I8F8))),
            Err(LimitError::LowOutOfRange)
        );
        assert_eq!(
            <Limits as TryFrom<(I8F8, I8F8)>>::try_from((fixed!(0.5: I8F8), fixed!(-0.5: I8F8))),
            Err(LimitError::LowExceedsHigh)
        );
    }
}
