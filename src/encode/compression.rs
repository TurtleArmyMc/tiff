use crate::types::Byte;

use super::{buffer::TiffEncodeBuffer, EncodeEndianness};

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
