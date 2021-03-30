//! Compression library for low-speed I2C sensors.
#![no_std]

use bitvec::prelude::*;
// mod stream;

pub type Packet = BitArray<Msb0, [u8; 2]>;

pub enum EntryType {
    Diff((i16, i16)),
    Absolute(i16),
    NoMeasurement,
    Reserved(i16),
}

#[repr(u8)]
pub enum Opcode {
    /// The measurement is the same as the previous measurement.
    Zero = 0b000,
    /// The measurement is one greater than the previous.
    Incr = 0b100,
    /// The measurement is one lesser than the previous.
    Decr = 0b101,
    /// The measurement is this payload.
    Item = 0b110,
    /// The measurement is the previous plus this payload.
    Diff = 0b111,
}

impl From<u8> for Opcode {
    fn from(val: u8) -> Self {
        match val {
            v if v == Self::Zero as u8 => Self::Zero,
            v if v == Self::Incr as u8 => Self::Incr,
            v if v == Self::Decr as u8 => Self::Decr,
            v if v == Self::Item as u8 => Self::Item,
            v if v == Self::Diff as u8 => Self::Diff,
            v => panic!("Don't send bad opcodes! {}", v),
        }
    }
}

pub fn encode_stream<'a, 'b>(
    mut values: &'a [i16],
    buf: &'b mut BitSlice<Msb0, <u8 as BitStore>::Alias>,
) -> (
    //  Measurements not serialized
    &'a [i16],
    //  Datastream for transport
    &'b BitSlice<Msb0, <u8 as BitStore>::Alias>,
    //  Unused datastream
    &'b mut BitSlice<Msb0, <u8 as BitStore>::Alias>,
) {
    let mut cursor = 0;
    let mut last = None;

    while let Some((next, rest)) = values.split_first() {
        let buf_len = buf.len();

        if buf_len <= cursor + 15 {
            break;
        }
        let entry = match last {
            None => EntryType::Absolute(*next),
            Some(last) => EntryType::Diff((*next, last)),
        };
        let (bits, pkt) = compress_entry(entry);
        buf[cursor..][..bits].clone_from_bitslice(&pkt[..bits]);

        last = Some(*next);
        cursor += bits;
        values = rest;
    }

    let (written, rest) = buf.split_at_mut(cursor);
    (values, &*written, rest)
}

pub fn decode_stream<'a, 'b>(
    mut data: &'a BitSlice<Msb0, u8>,
    values: &'b mut [i16],
) -> (
    //  Datastream not parsed
    &'a BitSlice<Msb0, u8>,
    //  Measurements deserialized
    &'b [i16],
    //  Unused measurements
    &'b mut [i16],
) {
    let mut cursor = 0;
    let mut last = None;

    for slot in values.iter_mut() {
        let data_len = data.len();

        if data_len < 1 {
            break;
        }
        if !data[0] {
            *slot = last.take().unwrap_or_default();
            data = &data[1..];
            cursor += 1;
            continue;
        }

        if data_len < 15 {
            break;
        }
        match data[..3].load_be::<u8>().into() {
            Opcode::Incr => {
                let mut prev = last.take().unwrap_or_default();
                prev += 1;
                last = Some(prev);
                *slot = prev;
                data = &data[3..];
            }
            Opcode::Decr => {
                let mut prev = last.take().unwrap_or_default();
                prev -= 1;
                last = Some(prev);
                *slot = prev;
                data = &data[3..];
            }
            Opcode::Item => {
                let val = if data[3] {
                    (data[3..15].load_be::<u16>() as i16) - 0x1000
                } else {
                    (data[3..15].load_be::<u16>() as i16)
                };

                last = Some(val);
                *slot = val;
                data = &data[15..];
            }
            Opcode::Diff => {
                let diff = if data[3] {
                    (data[3..15].load_be::<u16>() as i16) - 0x1000
                } else {
                    (data[3..15].load_be::<u16>() as i16)
                };

                let prev = last.take().unwrap_or_default();
                let next = prev + diff;
                last = Some(next);
                *slot = next;
                data = &data[15..];
            }
            Opcode::Zero => unreachable!("Handled earlier"),
        }

        cursor += 1;
    }

    let (read, rest) = values.split_at_mut(cursor);
    (data, &*read, rest)
}

fn compress_entry(entry: EntryType) -> (usize, Packet) {
    let mut out = Packet::zeroed();

    match entry {
        EntryType::Diff((curr, prev)) => {
            let diff = curr - prev;

            match diff {
                0 => {
                    out[..3].store(Opcode::Zero as u8);
                    (1, out)
                }
                1 => {
                    out[..3].store(Opcode::Incr as u8);
                    (3, out)
                }
                -1 => {
                    out[..3].store(Opcode::Decr as u8);
                    (3, out)
                }
                d if d > -2048 && d < 2048 => {
                    out[..3].store(Opcode::Diff as u8);
                    out[3..15].store_be(d as u16);
                    (15, out)
                }
                _ => panic!("Difference between consecutive measurements exceeds 2048."),
            }
        }
        EntryType::Absolute(val) => {
            out[..3].store(Opcode::Item as u8);
            out[3..15].store_be(val as u16);
            (15, out)
        }
        EntryType::NoMeasurement => (15, Packet::new([0xF0, 0])),
        EntryType::Reserved(val) => {
            if val >= -1 && val < 2 {
                out[..3].store(Opcode::Diff as u8);
                out[3..15].store(val as u16);
            } else {
                panic!("Reserved value out of range.")
            }
            (15, out)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as compress;
    use bitvec::prelude::*;
    use core::cell::Cell;

    #[test]
    fn test_zero() {
        let (s, b) = compress::compress_entry(compress::EntryType::Diff((1023, 1023)));
        assert_eq!((s, b.into_inner()), (1, [0, 0]));
    }

    #[test]
    fn test_delta1() {
        let (s, b) = compress::compress_entry(compress::EntryType::Diff((1023, 1022)));
        assert_eq!((s, b.into_inner()), (3, [0b10000000, 0]));
        let (s, b) = compress::compress_entry(compress::EntryType::Diff((1022, 1023)));
        assert_eq!((s, b.into_inner()), (3, [0b10100000, 0]));
    }

    #[test]
    fn test_delta12() {
        // 3 Delta
        let (s, b) = compress::compress_entry(compress::EntryType::Diff((1023, 1020)));
        assert_eq!((s, b.into_inner()), (15, [0b11100000, 0b00000110]));
        // -3 Delta
        let (s, b) = compress::compress_entry(compress::EntryType::Diff((1020, 1023)));
        assert_eq!((s, b.into_inner()), (15, [0b11111111, 0b11111010]));
    }

    #[test]
    fn test_item() {
        let (s, b) = compress::compress_entry(compress::EntryType::Absolute(1));
        assert_eq!((s, b.into_inner()), (15, [0b11000000, 0b00000010]));
    }

    #[test]
    fn encode_values() {
        let values = [1500, 0, 0, 1, 0, -1, 1000, 1001, 1000, 999, 500, 500];
        let mut buf = bitarr![Msb0, u8; 0; 256];
        let (_, buf_slice) = buf.as_mut_bitslice().split_at_mut(0);
        let (unencoded, stream, empty) = compress::encode_stream(&values, buf_slice);
        assert!(unencoded.is_empty());
        assert_eq!(
            stream,
            bits![Msb0, u8;
                // item: 1500
                1, 1, 0, /**/ 0, 1, 0, 1, /**/ 1, 1, 0, 1, /**/ 1, 1, 0, 0,
                // diff: -1500
                1, 1, 1, /**/ 1, 0, 1, 0, /**/ 0, 0, 1, 0, /**/ 0, 1, 0, 0,
                // zero diff
                0,
                // incr
                1, 0, 0,
                // decr
                1, 0, 1,
                // decr
                1, 0, 1,
                // diff: 1001
                1, 1, 1, /**/ 0, 0, 1, 1, /**/ 1, 1, 1, 0, /**/ 1, 0, 0, 1,
                // incr
                1, 0, 0,
                // decr
                1, 0, 1,
                // decr
                1, 0, 1,
                // diff: -499
                1, 1, 1, /**/ 1, 1, 1, 0, /**/ 0, 0, 0, 0, /**/ 1, 1, 0, 1,
                // zero diff
                0,
            ]
        );
    }

    #[test]
    fn decode_values() {
        let bits = bits![Msb0, u8;
            // item: 1500
            1, 1, 0, /**/ 0, 1, 0, 1, /**/ 1, 1, 0, 1, /**/ 1, 1, 0, 0,
            // diff: -1500
            1, 1, 1, /**/ 1, 0, 1, 0, /**/ 0, 0, 1, 0, /**/ 0, 1, 0, 0,
            // zero diff
            0,
            // incr
            1, 0, 0,
            // decr
            1, 0, 1,
            // decr
            1, 0, 1,
            // diff: 1001
            1, 1, 1, /**/ 0, 0, 1, 1, /**/ 1, 1, 1, 0, /**/ 1, 0, 0, 1,
            // incr
            1, 0, 0,
            // decr
            1, 0, 1,
            // decr
            1, 0, 1,
            // diff: -499
            1, 1, 1, /**/ 1, 1, 1, 0, /**/ 0, 0, 0, 0, /**/ 1, 1, 0, 1,
            // zero diff
            0,
        ];
        let mut buf = [0; 12];
        let (undecoded, stream, empty) = compress::decode_stream(&bits, &mut buf);
        assert!(undecoded.is_empty());
        assert_eq!(stream, [1500, 0, 0, 1, 0, -1, 1000, 1001, 1000, 999, 500, 500]);
    }
}
