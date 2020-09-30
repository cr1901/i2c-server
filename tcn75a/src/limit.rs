use core::convert::{From, TryFrom};

#[derive(Debug, PartialEq)]
pub struct Limits(i16, i16);

#[derive(Debug, PartialEq)]
pub enum LimitError {
    BothOutOfRange,
    LowOutOfRange,
    HighOutOfRange,
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
