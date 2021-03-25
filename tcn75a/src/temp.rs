use core::fmt;

use fixed::types::I8F8;

/** A struct representing a temperature reading from the TCN75A.

# Invariants

[`Temperature`] is a [newtype] over the [`FixedI16::<U8>`] ([`I8F8`]) type provided by the
[`fixed`] crate. The user cannot create this type; it mainly exists to provide a runtime
guarantee that the contained data was successfully read from the TCN75A.

# Examples

To compare, add, subtract, compare, etc temperature measurements from a TCN75A, you should convert
a [`Temperature`] to a [`I8F8`] type. [`Temperature`] implements [`Copy`], so a [`Temperature`]
can be used simultaneously with its contained [`I8F8`].

```
# cfg_if::cfg_if! {
# if #[cfg(any(target_os = "linux", target_os = "android"))] {
# use linux_embedded_hal::I2cdev;
# use embedded_hal::blocking::i2c::{Read, Write};
# use tcn75a::{Tcn75a, Tcn75aError, ConfigReg, Resolution};
# fn main() -> Result<(), Tcn75aError<I2cdev, I2cdev>> {
# let i2c = I2cdev::new("/dev/i2c-1").unwrap();
# let mut tcn = Tcn75a::new(i2c, 0x48);
use fixed::types::I8F8;
let temp0: I8F8 = tcn.temperature()?.into();
// ... Assume some time has passed.
let temp1: I8F8 = tcn.temperature()?.into();

if temp0 < temp1 {
    println!("Temperature is going up.");
} else if temp0 > temp1 {
    println!("Temperature is going down.");
} else {
    println!("Temperature is constant.");
}
# Ok(())
# }
# } else {
# fn main() {
# }
# }
# }
```

[`Temperature`]: ./struct.Temperature.html
[newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
[`FixedI16::<U8>`]: ../fixed/struct.FixedI16.html
[`I8F8`]: ../fixed/types/type.I8F8.html
[`fixed`]: ../fixed/index.html
[`Copy`]: https://doc.rust-lang.org/nightly/core/marker/trait.Copy.html
*/
#[derive(Debug, Clone, Copy)]
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
