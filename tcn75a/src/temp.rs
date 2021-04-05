use fixed::types::I8F8;

/** A struct representing a temperature reading from the TCN75A.

# Internals

[`Temperature`] is a [newtype] over the [`FixedI16::<U8>`] ([`I8F8`]) type provided by the
[`fixed`] crate.

# Invariants

The user cannot create this type; it mainly exists to provide a runtime
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
use fixed_macro::fixed;

let temp0: I8F8 = tcn.temperature()?.into();
// ... Assume some time has passed.
let baseline: I8F8 = fixed!(25.0 : I8F8);

if temp0 < baseline {
    println!("Temperature is less than 25.0C.");
} else if temp0 > baseline {
    println!("Temperature is greater than 25.0C.");
} else {
    println!("Temperature is 25.0C.");
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
