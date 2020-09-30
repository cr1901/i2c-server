use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

pub struct UnimplementedHal;

impl Read for UnimplementedHal {
    type Error = ();

    fn read(&mut self, _address: u8, _buffer: &mut [u8]) -> Result<(), Self::Error> {
        Err(())
    }
}

impl Write for UnimplementedHal {
    type Error = ();

    fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> {
        Err(())
    }
}

impl WriteRead for UnimplementedHal {
    type Error = ();

    fn write_read(&mut self, _address: u8, _bytes: &[u8], _buffer: &mut [u8]) -> Result<(), Self::Error> {
        Err(())
    }
}

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android"))] {
        use linux_embedded_hal::I2cdev;
        pub type HalImpl = I2cdev;
    } else {
        pub type HalImpl = UnimplementedHal;
    }
}

pub fn setup() -> HalImpl {
    cfg_if::cfg_if! {
        if #[cfg(any(target_os = "linux", target_os = "android"))] {
            // FIXME: Should integration tests panic?
            I2cdev::new("/dev/i2c-1").unwrap()
        } else {
            UnimplementedHal {}
        }
    }
}
