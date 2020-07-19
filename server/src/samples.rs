use std::ops::Deref;
use std::time::{SystemTime, Duration};

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
