# `tcn75a`
[![crates.io](https://img.shields.io/crates/v/tcn75a.svg)](https://crates.io/crates/tcn75a)
[![](https://docs.rs/tcn75a/badge.svg)](https://docs.rs/tcn75a/)
[![Contact Me](https://img.shields.io/twitter/follow/cr1901.svg?label=Contact%20Me&&style=social)](https://twitter.com/cr1901)

`tcn75a` is an [Embedded HAL] crate for accessing [Microchip TCN75A][TCN75A]
temperature sensors over a (single controller) I2C bus. The TCN75A is a
four-register temperature sensor that is easy to set up and poll in a task.
All features should be supported.

This crate contains copious amounts of documentation and tries to optimize
the number of I2C transactions sent to the TCN75A via caching. Therefore, at
present _this crate does not work with multicontroller I2C buses_, though it
should be possible to add this at the cost of performance.

## Dev Boards

You can get a dev board for the `tcn75a` from Digilent called the [PMOD TMP3].
If you don't have a microcontroller or FPGA board with a PMOD connector, the
PMOD TMP3 can be connected to a breadboard using a [PMOD DIP].

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
 * MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[Embedded HAL]: https://github.com/rust-embedded/embedded-hal
[TCN75A]: https://www.microchip.com/wwwproducts/TCN75A
[PMOD TMP3]: https://store.digilentinc.com/pmod-tmp3-digital-temperature-sensor/
[PMOD DIP]: https://store.digilentinc.com/pmod-dip-dip-to-12-pin-pmod-adapter/
[LICENSE-APACHE]: http://www.apache.org/licenses/LICENSE-2.0
[LICENSE-MIT]: http://opensource.org/licenses/MIT
