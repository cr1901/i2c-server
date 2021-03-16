use core::fmt;
use core::ops::{Add, Sub};

use fixed::types::I8F8;

/* Invariant: temperature() will ensure lower bits are cleared for the
given resolution. */

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone, Copy)]
pub struct Temperature(pub(crate) I8F8);

impl From<Temperature> for I8F8 {
    fn from(temp: Temperature) -> Self {
        temp.0
    }
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
