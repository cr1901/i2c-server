#![no_std]

use bitvec::prelude::*;

// mod stream;

// 0 = Zero change
// 100 +1 change
// 101 -1 change
// 110 xxxxxxxxxxxx 12 bit absolute
// 111 sxxxxxxxxxxx 12 bit delta (except for values described below)
// Niche-filling:
// 111 100000000000 Impossible delta with 12-bit sensor. No value/no measurement taken this sample.
// 111 000000000001 Equivalent to 100. Reserved. Probably "clock went backwards".
// 111 000000000000 Equivalent to 0. Reserved. Probably "long term jitter error".
// 111 111111111111 Equivalent to 101. Reserved. Probably "user event".

pub type Packet = BitArray<Msb0, [u8; 2]>;

pub enum EntryType {
    Diff((i16, i16)),
    Absolute(i16),
    NoMeasurement,
    Reserved(i16),
}

#[repr(u8)]
pub enum Opcode {
    /// The measurement is zero.
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

        if buf_len <= cursor {
            break;
        }
        if *next == 0 {
            buf.set(cursor, false);
            cursor += 1;
            last = Some(0);
            values = rest;
            continue;
        }

        if buf_len <= cursor + 15 {
            break;
        }
        let entry = match last {
            None => EntryType::Absolute(*next),
            Some(last) => EntryType::Diff((*next, last)),
        };
        let (bits, pkt) = compress_entry(entry);
        buf[cursor..][..bits].clone_from_bitslice(&pkt[..bits]);

        cursor += bits;
        values = rest;
    }

    let (written, rest) = buf.split_at_aliased_mut(cursor);
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
            *slot = 0;
            data = &data[1..];
            last = Some(0);
            cursor += 1;
            continue;
        }

        if data_len < 15 {
            break;
        }
        match data[..3].load::<u8>().into() {
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
                let val = data[3..15].load_be::<u16>() as i16;
                last = Some(val);
                *slot = val;
                data = &data[15..];
            }
            Opcode::Diff => {
                let diff = data[3..15].load_be::<u16>() as i16;
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

    // #[test]
    // fn test_zero() {
    //     // assert_eq!(compress::compress_entry(compress::EntryType::Diff((1023, 1023))), (1, bitarr![Msb0, u8; 0; 16]));
    //     // assert_eq!(compress::compress_entry(compress::EntryType::Diff((1023, 1023))), (1, bitarr![Lsb0, u8; 0; 16]));
    // }
    //
    // #[test]
    // fn test_delta1() {
    //     let (s, b) = compress::compress_entry(compress::EntryType::Diff((1023, 1022)));
    //     assert_eq!((s, b.load::<u16>()), (3, 0b10000000));
    //     let (s, b) = compress::compress_entry(compress::EntryType::Diff((1022, 1023)));
    //     assert_eq!((s, b.load::<u16>()), (3, 0b10100000));
    // }

    #[test]
    fn test_delta12() {
        // 111 sxxxxxxxxxxx 12 bit delta
        // I wanted bits stored in the above order (MSbit to LSbit in each byte):
        // High bit of                                                   Low bit of
        // Low byte => p2.p1.p0.s.x10.x9.x8.x7_x6.x5.x4.x3.x2.x1.x0.e <= High byte
        // e for "extra" bit to make 16-bits. The next data begins at "e".
        //
        // Why do I want the data to be stored like this?
        // So when read I read the data in a hex editor, you see: 1110 0000 0000 011x, or E0 06/7.
        // This corresponds how the value is written in the spec from left to right! Easier to
        // debug this way.
        //
        // Lsb0 order seems to be:
        // High bit of                                                   Low bit of
        // Low byte => x4.x3.x2.x1.x0.p2.p1.p0_e.s.x10.x9.x8.x7.x6.x5 <= High byte
        // "e" is a sign extension.
        //
        // Msb0 order seems to be- no idea, doesn't make sense to me...

        // 3 Delta
        let (s, b) = compress::compress_entry(compress::EntryType::Diff((1023, 1020)));
        assert_eq!((s, b.unwrap()), (15, [0b11100000, 0b00000110]));
        // -3 Delta
        let (s, b) = compress::compress_entry(compress::EntryType::Diff((1020, 1023)));
        assert_eq!((s, b.unwrap()), (15, [0b11111111, 0b11111010]));
    }

    #[test]
    fn encode_values() {
        let values = [1500, 0, 1, 0, -1, 1000, 1001, 1000, 999, 500];
        let mut buf = bitarr![Msb0, u8; 0; 256];
        let (_, buf_slice) = buf.as_mut_bitslice().split_at_mut(0);
        let (unencoded, stream, empty) = compress::encode_stream(&values, buf_slice);
        assert!(unencoded.is_empty());
        assert_eq!(
            stream,
            bits![Msb0, u8;
                // item: 1500
                1, 1, 0, /**/ 0, 1, 0, 1, /**/ 1, 1, 0, 1, /**/ 1, 1, 0, 0,
                // zero
                0,
                // incr
                1, 0, 0,
                // zero
                0,
                // decr
                1, 0, 1,
                // diff: 1001
                1, 1, 1, /**/ 0, 0, 1, 1, /**/ 1, 1, 1, 0, /**/ 1, 0, 0, 1,
                // decr
                1, 0, 1,
                // decr
                1, 0, 1,
                // diff: -499
                1, 1, 1, /**/ 1, 1, 1, 0, /**/ 0, 0, 0, 0, /**/ 1, 1, 0, 1,
            ]
        );
    }
}
