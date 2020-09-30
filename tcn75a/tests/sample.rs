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
fn sample<T>(mut tcn: Tcn75a<T>) where T: Read + Write + WriteRead {
    // This test only works if you're in a room with temperature > 0C!
    assert!(tcn.temperature().unwrap_or(0) > 0);
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn unimplemented<T>(_tcn: Tcn75a<T>) where T: Read + Write + WriteRead {

}
