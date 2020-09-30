use core::convert::{From, TryFrom};

pub struct Limits(i16, i16);
// pub struct LowerLimit(i16);
// pub struct UpperLimit(i16);

#[derive(Debug, PartialEq)]
pub enum LimitError {
    LowOutOfRange,
    HighOutOfRange,
    LowExceedsHigh
}

// impl Limit {
//     fn lower
// }

impl TryFrom<(i16, i16)> for Limits {
    type Error = LimitError;

    fn try_from(val: (i16, i16)) -> Result<Self, Self::Error> {
        if val.0 < -256 || val.0 > 255 {
            Err(LimitError::LowOutOfRange)
        } else if val.1 < -256 || val.1 > 255 {
            Err(LimitError::LowOutOfRange)
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
