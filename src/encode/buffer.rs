use std::{io::Write, iter::repeat, marker::PhantomData, mem::size_of};

use byteorder::WriteBytesExt;

use crate::{
    ifd,
    types::{Byte, Long, Short, URational},
};

use super::{image_header, EncodeEndianness};

pub struct TiffEncodeBuffer<E: EncodeEndianness> {
    bytes: Vec<u8>,
    phantom: PhantomData<E>,
}

pub(crate) struct TiffHeaderEncodeBuffer<'a, E: EncodeEndianness>(
    &'a mut [u8; image_header::LEN],
    PhantomData<E>,
);

pub(crate) struct IFDEncodeBuffer<'a, E: EncodeEndianness>(&'a mut [u8], PhantomData<E>);

pub(crate) struct IFDEntryEncodeBuffer<'a, E: EncodeEndianness>(
    &'a mut [u8; ifd::Entry::LEN],
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
        let inx = self.align_and_get_len();
        // Write number of directory entries
        self.append_short(fields.try_into().unwrap());
        // Reserve space for directory fields
        self.bytes.extend(repeat(0).take(fields * ifd::Entry::LEN));
        // Write next IFD offset
        self.bytes.extend([0, 0, 0, 0]);

        inx
    }

    pub(crate) fn get_ifd_at(&mut self, inx: usize, fields: usize) -> IFDEncodeBuffer<'_, E> {
        let end = inx + ifd::get_len(fields);
        IFDEncodeBuffer(&mut self.bytes[inx..end], PhantomData)
    }

    pub(crate) fn append_ifd_value(&mut self, ifd_value: &ifd::Values) -> [u8; 4] {
        let mut offset = [0, 0, 0, 0];

        match ifd_value {
            ifd::Values::Bytes(bytes) => match bytes[..] {
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
            ifd::Values::ASCII(string) => {
                (&mut offset[..])
                    .write_u32::<E>(self.align_and_get_len().try_into().unwrap())
                    .unwrap();
                self.bytes.extend(string.as_bytes());
                // Termainting NUL char
                self.append_byte(0);
            }
            ifd::Values::Shorts(shorts) => {
                match shorts[..] {
                    [] => (),
                    [short] => (&mut offset[0..2]).write_u16::<E>(short).unwrap(),
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
            ifd::Values::Longs(longs) => {
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
            ifd::Values::Rationals(rationals) => {
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

    pub(crate) fn append_byte(&mut self, byte: Byte) {
        self.bytes.push(byte)
    }

    pub(crate) fn append_short(&mut self, short: Short) {
        self.bytes.write_u16::<E>(short).unwrap()
    }

    pub(crate) fn append_long(&mut self, long: Long) {
        self.bytes.write_u32::<E>(long).unwrap()
    }

    pub(crate) fn append_urational(&mut self, urational: URational) {
        self.bytes.write_u32::<E>(urational.numerator).unwrap();
        self.bytes.write_u32::<E>(urational.denominator).unwrap()
    }

    pub(crate) fn extend_bytes<I: Iterator<Item = Byte>>(&mut self, iter: I) {
        self.bytes.extend(iter)
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut u8> {
        self.bytes.get_mut(index)
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn align_and_get_len(&mut self) -> usize {
        if self.bytes.len() % 2 == 1 {
            self.append_byte(0)
        }
        self.len()
    }
}

impl<'a, E: EncodeEndianness> TiffHeaderEncodeBuffer<'a, E> {
    pub(crate) fn set_first_ifd_offset(&mut self, offset: Long) {
        (&mut self.0[4..8]).write_u32::<E>(offset).unwrap()
    }
}

impl<'a, E: EncodeEndianness> IFDEncodeBuffer<'a, E> {
    pub(crate) fn get_entry(&mut self, entry_num: usize) -> IFDEntryEncodeBuffer<'_, E> {
        let start = ifd::ENTRY_COUNT_LEN + entry_num * ifd::Entry::LEN;
        let end = start + ifd::Entry::LEN;
        IFDEntryEncodeBuffer((&mut self.0[start..end]).try_into().unwrap(), PhantomData)
    }

    pub(crate) fn set_next_ifd_offset(&mut self, offset: Long) {
        let buff_inx = self.0.len() - size_of::<Long>();
        (&mut self.0[buff_inx..]).write_u32::<E>(offset).unwrap()
    }
}

impl<'a, E: EncodeEndianness> IFDEntryEncodeBuffer<'a, E> {
    pub(crate) fn set_all(&mut self, entry: &ifd::Entry, value_offset: [u8; 4]) {
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
