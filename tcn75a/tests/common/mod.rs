use cfg_if::cfg_if;
use embedded_hal::i2c::{blocking::I2c, ErrorKind, ErrorType};

pub struct UnimplementedHal;

impl ErrorType for UnimplementedHal {
    type Error = ErrorKind;
}

impl I2c for UnimplementedHal {
    fn read(&mut self, _address: u8, _buffer: &mut [u8]) -> Result<(), Self::Error> {
        Err(ErrorKind::Other)
    }

    fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> {
        Err(ErrorKind::Other)
    }

    fn write_iter<B>(&mut self, _address: u8, _bytes: B) -> Result<(), Self::Error>
    where
        B: IntoIterator<Item = u8>,
    {
        Err(ErrorKind::Other)
    }

    fn write_read(
        &mut self,
        _address: u8,
        _bytes: &[u8],
        _buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        Err(ErrorKind::Other)
    }

    fn write_iter_read<B>(
        &mut self,
        _address: u8,
        _bytes: B,
        _buffer: &mut [u8],
    ) -> Result<(), Self::Error>
    where
        B: IntoIterator<Item = u8>,
    {
        Err(ErrorKind::Other)
    }

    fn transaction<'a>(
        &mut self,
        _address: u8,
        _operations: &mut [embedded_hal::i2c::blocking::Operation<'a>],
    ) -> Result<(), Self::Error> {
        Err(ErrorKind::Other)
    }

    fn transaction_iter<'a, O>(&mut self, _address: u8, _operations: O) -> Result<(), Self::Error>
    where
        O: IntoIterator<Item = embedded_hal::i2c::blocking::Operation<'a>>,
    {
        Err(ErrorKind::Other)
    }
}

cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android"))] {
        use linux_embedded_hal::I2cdev;
        pub type HalImpl = I2cdev;
    } else {
        pub type HalImpl = UnimplementedHal;
    }
}

pub fn setup() -> HalImpl {
    cfg_if! {
        if #[cfg(any(target_os = "linux", target_os = "android"))] {
            // FIXME: Should integration tests panic?
            I2cdev::new("/dev/i2c-1").unwrap()
        } else {
            UnimplementedHal {}
        }
    }
}
