# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2024-10-06

### Added

- `From<Temperature>` is implemented for `i16` and `f32`.

### Changed

All changes for this release are breaking changes:

- Use [embedded-hal] `1.0.0`.
  - All dependents of [embedded-hal] used by this crate are updated to an
    appropriate version.
  - `Tcn75aError` now contains one generic parameter instead of two. It is
     intended to hold an [`i2c::ErrorType::Error`](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/trait.ErrorType.html#associatedtype.Error).
- `OutOfRange` variant of `Tcn75aError` now contains the raw `i16` value read
  from the sensor.

## [0.1.0] - 2021-06-21

Initial Release.

[embedded-hal](https://github.com/rust-embedded/embedded-hal)

[Unreleased]: https://github.com/cr1901/i2c-server/compare/tcn75a-v0.2.0...HEAD
[0.2.0]: https://github.com/cr1901/i2c-server/compare/tcn75a-v0.1.0...tcn75a-v0.2.0
[0.1.0]: https://github.com/cr1901/i2c-server/releases/tag/tcn75a-v0.1.0
