use embedded_hal::blocking::i2c::{Read, Write, WriteRead};
use tcn75a::*;

mod common;

#[test]
fn test_sample() {
    let hal = common::setup();
    #[allow(unused_mut)]
    let mut tcn = Tcn75a::new(hal, 0x48);

    #[cfg(any(target_os = "linux", target_os = "android"))]
    sample(tcn);
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    unimplemented(tcn);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn sample<T>(mut tcn: Tcn75a<T>)
where
    T: Read + Write + WriteRead,
{
    let mut cfg = ConfigReg::new();
    cfg.set_resolution(Resolution::Bits9);
    assert!(tcn.set_config_reg(cfg).is_ok());

    // This test only works if you're in a room with temperature > 0C!
    let temp9 = tcn.temperature().unwrap_or(0);
    assert!(temp9 > 0);

    cfg.set_resolution(Resolution::Bits12);
    assert!(tcn.set_config_reg(cfg).is_ok());

    // Check that 12-bit temp has is within 0.5C of 9-bit temp.
    let temp12 = tcn.temperature().unwrap_or(0);
    assert!((temp9 + 8) >= temp12 && (temp9 - 8) <= temp12);
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn unimplemented<T>(_tcn: Tcn75a<T>)
where
    T: Read + Write + WriteRead,
{
}
