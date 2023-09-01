use std::collections::HashMap;

use crate::{colors, types::Byte};

use super::{buffer::TiffEncodeBuffer, EncodeEndianness};

pub trait Compression<C: colors::Color>: sealed::CompressionImpl {}

#[derive(Clone, Copy)]
pub struct NoCompression;
impl<C: colors::Color> Compression<C> for NoCompression {}

#[derive(Clone, Copy)]
pub struct PackBits;
impl<C: colors::Color> Compression<C> for PackBits {}

#[derive(Clone, Copy)]
pub struct Lzw;
impl<C: colors::Color> Compression<C> for Lzw {}

/// `If n is between 0 and 127 inclusive, copy the next n+1 bytes literally`
///
/// The max length as it will appear encoded (starting at 0 for length 1).
const MAX_ENCODED_LITERAL_RUN_LEN: u8 = 127;

/// `Else if n is between -127 and -1 inclusive, copy the next byte -n+1 times`
///
/// The max value for Run::len as it will appear before encoding
/// (starting at 1 for length 1).
const MAX_RUN_LEN: u8 = 128;

#[derive(Clone, Copy)]
struct Run {
    byte: Byte,
    len: u8,
}

/// Writes bytes using PackBit compression.
pub(crate) fn packbits<E: EncodeEndianness, I: Iterator<Item = Byte>>(
    wrt: &mut TiffEncodeBuffer<E>,
    iter: I,
) {
    // Index of the start (count byte) of the last run if the last run was a literal run
    let mut last_literal_run_inx: Option<usize> = None;

    // The current run of a byte
    let mut current_run: Option<Run> = None;

    for byte in iter {
        match current_run.as_mut() {
            Some(run) if byte == run.byte => {
                // Continuation of current run
                match run.len {
                    MAX_RUN_LEN.. => {
                        // Max replicate run length reached
                        encode_replicate_run(wrt, *run); // Write current run
                        last_literal_run_inx = None; // Last run was not a literal run
                        run.len = 1; // Start new run
                    }
                    _ => {
                        // Can increment run length
                        run.len += 1;
                    }
                }
            }
            Some(run) => {
                // Different byte from current run
                last_literal_run_inx = encode_run(wrt, *run, last_literal_run_inx);
                // Store the new current run
                current_run = Some(Run { byte, len: 1 });
            }
            // Start a new run
            None => current_run = Some(Run { byte, len: 1 }),
        }
    }

    // Encode last run
    if let Some(run) = current_run {
        encode_run(wrt, run, last_literal_run_inx);
    }
}

/// Encodes the current run and returns the index of the start of the last
/// literal run if the run was not encoded as a replicate run.
fn encode_run<E: EncodeEndianness>(
    wrt: &mut TiffEncodeBuffer<E>,
    run: Run,
    last_literal_run_inx: Option<usize>,
) -> Option<usize> {
    match run.len {
        // Encode current run
        0..=1 => {
            // Current run is only 1 character and needs to be encoded
            // in a literal run
            match last_literal_run_inx.and_then(|i| wrt.get_mut(i)) {
                Some(literal_run_len) if *literal_run_len <= MAX_ENCODED_LITERAL_RUN_LEN - 1 => {
                    // The previous run was a literal run that can fit the byte
                    *literal_run_len += 1;
                    wrt.append_byte(run.byte);
                    last_literal_run_inx
                }
                _ => {
                    // A new literal run is needed
                    let new_literal_run_inx = Some(wrt.len()); // Index of new literal run
                    wrt.append_byte(0); // Start of a new 1 byte literal run
                    wrt.append_byte(run.byte);
                    new_literal_run_inx
                }
            }
        }
        2 => {
            // Encode a 2-byte repeat run as a replicate run except when preceded by a literal run
            match last_literal_run_inx.and_then(|i| wrt.get_mut(i)) {
                Some(literal_run_len) if *literal_run_len <= MAX_ENCODED_LITERAL_RUN_LEN - 2 => {
                    // There was a previous literal run that can fit the bytes
                    *literal_run_len += 2;
                    wrt.append_byte(run.byte);
                    wrt.append_byte(run.byte);
                    last_literal_run_inx
                }
                Some(literal_run_len) if *literal_run_len == MAX_ENCODED_LITERAL_RUN_LEN - 1 => {
                    // There was a previous literal run that can fit one byte
                    *literal_run_len += 1;
                    wrt.append_byte(run.byte);
                    // Create new literal run for second byte
                    let new_literal_run_inx = Some(wrt.len()); // Index of new literal run
                    wrt.append_byte(0); // Start of a new 1 byte literal run
                    wrt.append_byte(run.byte);
                    new_literal_run_inx
                }
                _ => {
                    // Encode as replicate run
                    encode_replicate_run(wrt, run);
                    None
                }
            }
        }
        2.. => {
            encode_replicate_run(wrt, run);
            None
        }
    }
}

fn encode_replicate_run<E: EncodeEndianness>(
    wrt: &mut TiffEncodeBuffer<E>,
    Run { byte, len: count }: Run,
) {
    // if n is between -127 and -1 inclusive, copy the next byte -n+1 times.
    wrt.append_byte((-((count - 1) as i8)) as u8);
    wrt.append_byte(byte);
}

/// Packs pairs of 4bit numbers into a single byte. Assumes that only the lowest
/// 4 bits of each number are non-zero. Highest order bits filled first.
pub(crate) struct HalfBytePacker<I: Iterator<Item = Byte>>(I);

impl<I: Iterator<Item = Byte>> HalfBytePacker<I> {
    pub(crate) fn new(iter: I) -> Self {
        Self(iter)
    }
}

impl<I: Iterator<Item = Byte>> Iterator for HalfBytePacker<I> {
    type Item = Byte;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|first| (first << 4) | self.0.next().unwrap_or_default())
    }
}

/// Packs 8 bits into a single byte. Highest order bits filled first.
pub(crate) struct BitPacker<I: Iterator<Item = bool>>(I);

impl<I: Iterator<Item = bool>> BitPacker<I> {
    pub(crate) fn new(iter: I) -> Self {
        Self(iter)
    }
}

impl<I: Iterator<Item = bool>> Iterator for BitPacker<I> {
    type Item = Byte;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|first| {
            let mut byte = (first as u8) << 7;
            for bit in (0..=6).rev() {
                byte |= (self.0.next().unwrap_or_default() as u8) << bit;
            }
            byte
        })
    }
}

mod sealed {
    use crate::{
        encode::{buffer::TiffEncodeBuffer, EncodeEndianness},
        ifd,
        types::Byte,
    };

    use super::{lzw, packbits, Lzw, NoCompression, PackBits};

    pub trait CompressionImpl {
        fn compression_type_tag(&self) -> ifd::tags::Compression;

        fn encode<I: Iterator<Item = Byte>, E: EncodeEndianness>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            iter: I,
        );
    }

    impl CompressionImpl for NoCompression {
        fn compression_type_tag(&self) -> ifd::tags::Compression {
            ifd::tags::Compression::NoCompression
        }

        fn encode<I: Iterator<Item = Byte>, E: EncodeEndianness>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            iter: I,
        ) {
            wrt.extend_bytes(iter)
        }
    }

    impl CompressionImpl for PackBits {
        fn compression_type_tag(&self) -> ifd::tags::Compression {
            ifd::tags::Compression::PackBits
        }

        fn encode<I: Iterator<Item = Byte>, E: EncodeEndianness>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            iter: I,
        ) {
            packbits(wrt, iter)
        }
    }

    impl CompressionImpl for Lzw {
        fn compression_type_tag(&self) -> ifd::tags::Compression {
            ifd::tags::Compression::Lzw
        }

        fn encode<I: Iterator<Item = Byte>, E: EncodeEndianness>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            iter: I,
        ) {
            lzw(wrt, iter)
        }
    }
}

/// Writes bytes using LZW compression.
pub(crate) fn lzw<E: EncodeEndianness, I: Iterator<Item = Byte>>(
    wrt: &mut TiffEncodeBuffer<E>,
    mut iter: I,
) {
    const CLEAR_CODE: u16 = 256;
    const END_OF_INFORMATION_CODE: u16 = 257;
    const FIRST_CODE: u16 = 258;
    // Codes should only be up to 12 bits
    const MAX_CODE: u16 = 4094;

    let mut bits = Vec::new();

    fn append_code(bits: &mut Vec<bool>, code: u16, bitcount: u8) {
        for i in (0..bitcount).rev() {
            bits.push(((code >> i) & 1) != 0);
        }
    }

    fn get_bitcount(string_table: &HashMap<Vec<u8>, u16>) -> u8 {
        match FIRST_CODE + string_table.len() as u16 {
            0..=255 => panic!(),
            256..=511 => 9,
            512..=1023 => 10,
            1024..=2047 => 11,
            2048..=MAX_CODE => 12,
            4095.. => panic!(),
        }
    }

    fn get_code(string_table: &HashMap<Vec<u8>, u16>, string: &[u8]) -> u16 {
        match string {
            [byte] => *byte as u16,
            string => string_table[string],
        }
    }

    fn add_entry(bits: &mut Vec<bool>, string_table: &mut HashMap<Vec<u8>, u16>, string: &[u8]) {
        let next_code = FIRST_CODE + string_table.len() as u16;
        string_table.insert(string.into(), next_code);
        if next_code == MAX_CODE {
            append_code(bits, CLEAR_CODE, 12);
            string_table.clear();
        }
    }

    let mut curr = Vec::new();
    curr.push(match iter.next() {
        Some(byte) => byte,
        None => panic!(),
    });
    let mut string_table: HashMap<Vec<u8>, u16> = HashMap::new();

    append_code(&mut bits, CLEAR_CODE, 9);

    for byte in iter {
        curr.push(byte);
        if curr.len() > 1 && !string_table.contains_key(&curr) {
            let code = get_code(&string_table, &curr[..curr.len() - 1]);
            append_code(&mut bits, code, get_bitcount(&string_table));
            add_entry(&mut bits, &mut string_table, &curr);
            curr.clear();
            curr.push(byte);
        }
    }

    let code = get_code(&string_table, &curr);
    let bitcount = get_bitcount(&string_table);
    append_code(&mut bits, code, bitcount);
    append_code(&mut bits, END_OF_INFORMATION_CODE, bitcount);

    wrt.extend_bytes(BitPacker::new(bits.into_iter()));
}
