use std::{io::Write, iter::repeat, marker::PhantomData};

use byteorder::WriteBytesExt;

use crate::{
    ifd::ifd::{self, IFDEntry, IfdFieldValues},
    types::{Byte, Long, Short, URational},
};

use super::{image_header, EncodeEndianness};

pub(crate) struct TiffEncodeBuffer<E: EncodeEndianness> {
    bytes: Vec<u8>,
    phantom: PhantomData<E>,
}

pub(crate) struct TiffHeaderEncodeBuffer<'a, E: EncodeEndianness>(
    &'a mut [u8; image_header::LEN],
    PhantomData<E>,
);

pub(crate) struct IFDEncodeBuffer<'a, E: EncodeEndianness>(&'a mut [u8], PhantomData<E>);

pub(crate) struct IFDEntryEncodeBuffer<'a, E: EncodeEndianness>(
    &'a mut [u8; IFDEntry::LEN_BYTES],
    PhantomData<E>,
);

impl<E: EncodeEndianness> TiffEncodeBuffer<E> {
    pub(crate) fn new() -> Self {
        let mut ret = Self {
            bytes: vec![E::get_sentinel(), E::get_sentinel()], // Endianness
            phantom: PhantomData,
        };
        // Magic number
        ret.append_short(42);
        // Byte offset of first IFD (immediately after the header). This can be
        // overwritten later if necessary.
        ret.append_long(8);
        ret
    }

    pub(crate) fn to_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn get_tiff_header(&mut self) -> TiffHeaderEncodeBuffer<'_, E> {
        TiffHeaderEncodeBuffer(
            (&mut self.bytes[0..image_header::LEN]).try_into().unwrap(),
            PhantomData,
        )
    }

    pub(crate) fn append_new_ifd(&mut self, fields: usize) -> usize {
        self.ensure_aligned();
        let inx = self.len();
        // Write number of directory entries
        self.append_short(fields.try_into().unwrap());
        // Reserve space for directory fields
        self.bytes
            .extend(repeat(0).take(fields * IFDEntry::LEN_BYTES));
        // Write next IFD offset
        self.bytes.extend([0, 0, 0, 0]);

        inx
    }

    pub(crate) fn get_ifd_at(&mut self, inx: usize, fields: usize) -> IFDEncodeBuffer<'_, E> {
        let end =
            inx + ifd::ENTRY_COUNT_LEN + ifd::NEXT_IFD_OFFSET_LEN + fields * IFDEntry::LEN_BYTES;
        IFDEncodeBuffer(&mut self.bytes[inx..end], PhantomData)
    }

    pub(crate) fn append_ifd_value(&mut self, ifd_value: &IfdFieldValues) -> [u8; 4] {
        let mut offset = [0, 0, 0, 0];

        match ifd_value {
            IfdFieldValues::Bytes(bytes) => match bytes[..] {
                [] | [_] | [_, _] | [_, _, _] | [_, _, _, _] => {
                    (&mut offset[..]).write(&bytes).unwrap();
                }
                _ => {
                    (&mut offset[..])
                        .write_u32::<E>(self.align_and_get_len().try_into().unwrap())
                        .unwrap();
                    self.bytes.extend(bytes.iter());
                }
            },
            IfdFieldValues::ASCII(string) => {
                (&mut offset[..])
                    .write_u32::<E>(self.align_and_get_len().try_into().unwrap())
                    .unwrap();
                self.bytes.extend(string.as_bytes());
                // Termainting NUL char
                self.append_byte(0);
            }
            IfdFieldValues::Shorts(shorts) => {
                match shorts[..] {
                    [] => (),
                    [short] => (&mut offset[..2]).write_u16::<E>(short).unwrap(),
                    [short1, short2] => {
                        (&mut offset[0..2]).write_u16::<E>(short1).unwrap();
                        (&mut offset[2..4]).write_u16::<E>(short2).unwrap();
                    }
                    _ => {
                        (&mut offset[..])
                            .write_u32::<E>(self.align_and_get_len().try_into().unwrap())
                            .unwrap();
                        for short in shorts.iter() {
                            self.append_short(*short);
                        }
                    }
                };
            }
            IfdFieldValues::Longs(longs) => {
                match longs[..] {
                    [] => (),
                    [long] => (&mut offset[..]).write_u32::<E>(long).unwrap(),
                    _ => {
                        (&mut offset[..])
                            .write_u32::<E>(self.align_and_get_len().try_into().unwrap())
                            .unwrap();
                        for long in longs.iter() {
                            self.append_long(*long);
                        }
                    }
                };
            }
            IfdFieldValues::Rationals(rationals) => {
                (&mut offset[..])
                    .write_u32::<E>(self.align_and_get_len().try_into().unwrap())
                    .unwrap();
                for rat in rationals.iter() {
                    self.append_urational(*rat);
                }
            }
        }

        offset
    }

    pub(crate) fn append_byte_aligned(&mut self, byte: Byte) {
        self.ensure_aligned();
        self.append_byte(byte)
    }

    pub(crate) fn append_byte(&mut self, byte: Byte) {
        self.bytes.push(byte)
    }

    pub(crate) fn append_short_aligned(&mut self, short: Short) {
        self.ensure_aligned();
        self.append_short(short)
    }

    pub(crate) fn append_short(&mut self, short: Short) {
        self.bytes.write_u16::<E>(short).unwrap()
    }

    pub(crate) fn append_long_aligned(&mut self, long: Long) {
        self.ensure_aligned();
        self.append_long(long)
    }

    pub(crate) fn append_long(&mut self, long: Long) {
        self.bytes.write_u32::<E>(long).unwrap()
    }

    pub(crate) fn append_urational_aligned(&mut self, urational: URational) {
        self.ensure_aligned();
        self.append_urational(urational)
    }

    pub(crate) fn append_urational(&mut self, urational: URational) {
        self.bytes.write_u32::<E>(urational.numerator).unwrap();
        self.bytes.write_u32::<E>(urational.denominator).unwrap()
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn align_and_get_len(&mut self) -> usize {
        self.ensure_aligned();
        self.len()
    }

    pub(crate) fn is_aligned(&self) -> bool {
        self.bytes.len() % 2 == 0
    }

    fn ensure_aligned(&mut self) {
        if !self.is_aligned() {
            self.append_byte(0)
        }
    }
}

impl<'a, E: EncodeEndianness> TiffHeaderEncodeBuffer<'a, E> {
    pub(crate) fn set_first_ifd_offset(&mut self, offset: Long) {
        (&mut self.0[4..8]).write_u32::<E>(offset).unwrap()
    }
}

impl<'a, E: EncodeEndianness> IFDEncodeBuffer<'a, E> {
    pub(crate) fn get_entry(&mut self, entry_num: usize) -> IFDEntryEncodeBuffer<'_, E> {
        let start = ifd::ENTRY_COUNT_LEN + entry_num * IFDEntry::LEN_BYTES;
        let end = start + IFDEntry::LEN_BYTES;
        IFDEntryEncodeBuffer((&mut self.0[start..end]).try_into().unwrap(), PhantomData)
    }

    pub(crate) fn set_next_ifd_offset(&mut self, offset: Long) {
        (&mut self.0).write_u32::<E>(offset).unwrap()
    }
}

impl<'a, E: EncodeEndianness> IFDEntryEncodeBuffer<'a, E> {
    pub(crate) fn set_all(&mut self, entry: &IFDEntry, value_offset: [u8; 4]) {
        // Write tag
        (&mut self.0[0..2])
            .write_u16::<E>(entry.tag() as u16)
            .unwrap();
        // Write value type
        (&mut self.0[2..4])
            .write_u16::<E>(entry.values().field_type_tag() as u16)
            .unwrap();
        // Write number of values
        (&mut self.0[4..8])
            .write_u32::<E>(entry.values().num_values())
            .unwrap();
        // Write number of values
        (&mut self.0[8..12]).write(&value_offset).unwrap();
    }
}
