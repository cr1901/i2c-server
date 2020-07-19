use std::ops::Deref;
use std::time::{SystemTime, Duration};

use base64::{encode_config_slice, URL_SAFE};
use serde::{Serialize, Deserialize, Serializer, Deserializer, ser::SerializeStruct};
use slice_deque::SliceDeque;

pub struct SampleBuf<T> {
    timestamp: u64,
    sample_rate: u8,
    buf: SliceDeque<T>
}

impl<T> SampleBuf<T> {
    pub fn new(capacity : usize, sample_rate: u8) -> Self {
        SampleBuf {
            timestamp: 0,
            sample_rate: sample_rate,
            buf: SliceDeque::with_capacity(capacity)
        }
    }

    pub fn post(&mut self, now: SystemTime, sample: T) -> Result<(), ()> {
        let prev_systime = SystemTime::UNIX_EPOCH + Duration::from_secs(self.timestamp);

        let leap_sec = match now.duration_since(prev_systime) {
            Ok(dur) => {
                // Posting a new measurement does not support zero duration between
                // measurements.
                self.timestamp = now.duration_since(SystemTime::UNIX_EPOCH).map_err(|_| ())?.as_secs();
                dur == Duration::new(0, 0)
            },
            Err(_e) => {
                // TODO: We can handle up to 1 second duration in the past of samples.
                // We should fail if the system clock was updated, however.
                // Also handle case where elapsed time is >= 2 sampling times since
                // previous sample.
                true
            }
        };

        if self.buf.is_full() {
            self.buf.pop_front();
        }

        if !leap_sec {
            self.buf.push_back(sample);
        } else {
            // TODO: Leap "seconds" will be encoded specially when compression
            // is implemented.
            self.buf.push_back(sample);
        }

        Ok(())
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

impl<T> Deref for SampleBuf<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl<T> Serialize for SampleBuf<T> where T: private::Sealed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("SampleBuf", 3)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.serialize_field("sample_rate", &self.sample_rate)?;

        let max_base64_size = self.capacity() * 4 / 3 + 4;
        let mut payload = Vec::<u8>::with_capacity(max_base64_size);
        payload.resize(max_base64_size, 0);

        let byte_data = {
            let temp_ptr = &**self as *const [T] as *const T as *const u8;
            unsafe { std::slice::from_raw_parts(temp_ptr, self.len() * 2) }
        };

        let written = encode_config_slice(byte_data, URL_SAFE, &mut payload);
        payload.resize(written, 0);

        // Base64 data will already be ASCII, which is UTF-8 subset.
        let payload_string = unsafe { String::from_utf8_unchecked(payload) };
        state.serialize_field("buf", &payload_string)?;
        state.end()
    }
}

mod private {
    pub trait Sealed {}

    // The Serializer for my intended use case uses unsafe code to convert to a
    // base64 string. I don't feel like trying to prove it's safe for arbitrary
    // types, so limit to the ints and floats for now.
    impl Sealed for usize {}
    impl Sealed for u128 {}
    impl Sealed for u64 {}
    impl Sealed for u32 {}
    impl Sealed for u16 {}
    impl Sealed for u8 {}
    impl Sealed for isize {}
    impl Sealed for i128 {}
    impl Sealed for i64 {}
    impl Sealed for i32 {}
    impl Sealed for i16 {}
    impl Sealed for i8 {}
    impl Sealed for f64 {}
    impl Sealed for f32 {}
}
