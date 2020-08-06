#![no_std]
use bitvec::array::BitArray;
use bitvec::prelude::*;

mod stream;

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

pub enum EntryType {
    Diff((i16, i16)),
    Absolute(i16),
    NoMeasurement,
    Reserved(i16),
}

mod entry_prefix {
    pub const ZERO: u8 = 0;
    pub const DELTA_1: u8 = 2;
    pub const ABS: u8 = 6;
    pub const DELTA_12: u8 = 7;
    pub const RESERVED: u8 = 7;
}

pub fn compress<O>(in_buf: &[i16], out_buf: &mut BitSlice<O, u8>, start_implied: bool)
where
    O: BitOrder,
{
    todo!()
    // let entries = if start_implied {
    //     // (curr, tmp) = in_buf.split_at(1);
    //     // tmp
    //      // = in_buf.0;
    // } else {
    //     // in_buf
    // };
    //
    // for entry in entries {
    //     // match entry {
    //     //
    //     // }
    // }
}

pub fn decompress() {
    todo!()
}

fn compress_entry<O>(entry: EntryType) -> (usize, BitArray<O, [u8; 2]>)
where
    O: BitOrder,
    BitSlice<O, u8>: BitField,
    // BitSlice<O, u8>: BitField + IndexMut<usize>, Eeep! 25 Errors!
{
    let mut out = BitArray::zeroed();

    match entry {
        EntryType::Diff((curr, prev)) => {
            let diff = curr - prev;

            match diff {
                0 => {
                    out[0..1].store(entry_prefix::ZERO);
                    (1usize, out)
                },
                1 => {
                    out[..2].store(entry_prefix::DELTA_1);
                    out[2..3].store(0u16);
                    (3usize, out)
                },
                -1 => {
                    out[..2].store(entry_prefix::DELTA_1);
                    out[2..3].store(1u16);
                    (3usize, out)
                }
                d if d > -2048 && d < 2048 => {
                    out[0..3].store(entry_prefix::DELTA_12);
                    out[3..16].store(d as u16);
                    (15usize, out)
                },
                _ => panic!("Difference between consecutive measurements exceeds 2048.")
            }
        }
        EntryType::Absolute(val) => {
            out[..3].store(entry_prefix::ABS);
            out[3..16].store(val as u16);
            (15usize, out)
        }
        EntryType::NoMeasurement => {
            out[..3].store(entry_prefix::RESERVED);
            out[3..16].store(-2048i16 as u16);
            (15usize, out)
        }
        EntryType::Reserved(val) => {
            if val >= -1 && val < 2 {
                out[..3].store(entry_prefix::RESERVED);
                out[3..16].store(val as u16);
            } else {
                panic!("Reserved value out of range.")
            }
            (15usize, out)
        }
        _ => todo!(),
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
        let (s, b) = compress::compress_entry::<Lsb0>(compress::EntryType::Diff((1023, 1020)));
        assert_eq!((s, b.load::<u16>()), (15, 0b00000000_00011111));
        // -3 Delta
        let (s, b) = compress::compress_entry::<Lsb0>(compress::EntryType::Diff((1020, 1023)));
        assert_eq!((s, b.load::<u16>()), (15, 0b11111111_11101111));


        let (s, b) = compress::compress_entry::<Msb0>(compress::EntryType::Diff((1023, 1020)));
        assert_eq!((s, b.load::<u16>()), (15, 0b00000000_11100011));
        let (s, b) = compress::compress_entry::<Msb0>(compress::EntryType::Diff((1020, 1023)));
        assert_eq!((s, b.load::<u16>()), (15, 0b11111111_11111101));
    }
}
