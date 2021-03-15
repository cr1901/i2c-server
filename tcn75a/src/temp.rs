use core::fmt;
use core::ops::{Add, Sub};

use fixed::types::I8F8;

/* Invariant: temperature() will ensure lower bits are cleared for the
given resolution. */

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone, Copy)]
pub struct Temperature(pub(crate) I8F8);

impl From<Temperature> for f32 {
    fn from(temp: Temperature) -> Self {
        I8F8::to_num(temp.0)
    }
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Add<i8> for Temperature {
    type Output = Self;

    fn add(self, other: i8) -> Self::Output {
        Self(self.0 + I8F8::from_num(other))
    }
}

impl Add<I8F8> for Temperature {
    type Output = Self;

    fn add(self, other: I8F8) -> Self::Output {
        Self(self.0 + other)
    }
}
