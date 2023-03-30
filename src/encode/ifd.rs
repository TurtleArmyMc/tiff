use std::{io::Write, iter::repeat};

use byteorder::WriteBytesExt;

use crate::ifd::ifd::{IFDEntry, IfdFieldType, IfdFieldValues, URational};

use super::EncodeEndianness;

/// Encodes headers and values for entries
pub(crate) fn encode_ifds<E: EncodeEndianness>(wrt: &mut Vec<u8>, mut ifds: Vec<IFDEntry>) {
    if ifds.len() > u16::MAX as usize {
        todo!("add support for splitting IFD entries into multiple headers");
    }

    ifds.sort_unstable_by_key(|ifd| ifd.tag());
    // Write number of directory entries
    wrt.write_u16::<E>(ifds.len() as u16).unwrap();
    // Reserve space for directory fields
    let ifd_entry_inx_start = wrt.len();
    wrt.extend(repeat(0).take(ifds.len() * IFDEntry::LEN_BYTES));
    // Write next IFD offset
    wrt.extend([0, 0, 0, 0]);
    // Write entries
    for (inx, entry) in ifds.iter().enumerate() {
        let field_inx = ifd_entry_inx_start + (inx * IFDEntry::LEN_BYTES);
        entry.encode::<E>(wrt, field_inx);
    }
}

impl IFDEntry {
    pub fn encode<E: EncodeEndianness>(&self, wrt: &mut Vec<u8>, field_inx: usize) {
        const FIELD_TAG_LEN: usize = 2;

        // Write tag
        (&mut wrt[field_inx..field_inx + FIELD_TAG_LEN])
            .write_u16::<E>(self.tag() as u16)
            .unwrap();
        // Write values
        self.values().encode::<E>(wrt, field_inx);
    }
}

impl IfdFieldValues {
    pub fn encode<E: EncodeEndianness>(&self, wrt: &mut Vec<u8>, field_inx: usize) {
        const FIELD_TYPE_TAG_INX: usize = 2;
        const FIELD_TYPE_TAG_LEN: usize = 2;

        const NUM_VALUES_INX: usize = 4;
        const NUM_VALUES_LEN: usize = 4;

        const FIELD_VALUE_OFFSET_INX: usize = 8;
        const FIELD_VALUE_OFFSET_LEN: usize = 4;

        // If the field values will not fit within the IFD field offset, then
        // they will be inserted at the end of the currently encoded file.
        // Because the offset must be on a word boundry, there will need to be
        // a byte of padding if the length of the encoded buffer is currently
        // odd. wrt can't be pre-emptively extended, because the values might
        // fit in the field's offset, and if it fits in there then wrt should
        // stay at the length it currently is.
        let (field_values_offset_value, field_offset_needs_padding) = if wrt.len() % 2 == 0 {
            (wrt.len() as u32, false)
        } else {
            (wrt.len() as u32 + 1, true)
        };

        let type_tag_inx = field_inx + FIELD_TYPE_TAG_INX;
        let mut type_tag_buff = &mut wrt[type_tag_inx..type_tag_inx + FIELD_TYPE_TAG_LEN];
        type_tag_buff
            .write_u16::<E>(self.field_type_tag() as u16)
            .unwrap();

        let num_values_inx = field_inx + NUM_VALUES_INX;
        let mut num_values_buff = &mut wrt[num_values_inx..num_values_inx + NUM_VALUES_LEN];
        num_values_buff.write_u32::<E>(self.num_values()).unwrap();

        let values_offset_inx = field_inx + FIELD_VALUE_OFFSET_INX;
        let mut field_values_offset_buff =
            &mut wrt[values_offset_inx..values_offset_inx + FIELD_VALUE_OFFSET_LEN];
        match self {
            IfdFieldValues::Bytes(bytes) => {
                if bytes.len() <= FIELD_VALUE_OFFSET_LEN {
                    field_values_offset_buff.write(&bytes).unwrap();
                } else {
                    field_values_offset_buff
                        .write_u32::<E>(field_values_offset_value)
                        .unwrap();
                    if field_offset_needs_padding {
                        wrt.push(0);
                    }
                    wrt.extend(bytes.iter());
                };
            }
            IfdFieldValues::ASCII(string) => {
                field_values_offset_buff
                    .write_u32::<E>(field_values_offset_value)
                    .unwrap();
                if field_offset_needs_padding {
                    wrt.push(0);
                }
                wrt.extend(string.as_bytes());
                // Termainting NUL char
                wrt.push(0);
            }
            IfdFieldValues::Shorts(shorts) => {
                match shorts[..] {
                    [short] => field_values_offset_buff.write_u16::<E>(short).unwrap(),
                    [short1, short2] => {
                        field_values_offset_buff.write_u16::<E>(short1).unwrap();
                        field_values_offset_buff.write_u16::<E>(short2).unwrap();
                    }
                    _ => {
                        field_values_offset_buff
                            .write_u32::<E>(field_values_offset_value)
                            .unwrap();
                        if field_offset_needs_padding {
                            wrt.push(0);
                        }
                        for short in shorts.iter() {
                            wrt.write_u16::<E>(*short).unwrap();
                        }
                    }
                };
            }
            IfdFieldValues::Longs(longs) => {
                match longs[..] {
                    [long] => field_values_offset_buff.write_u32::<E>(long).unwrap(),
                    _ => {
                        field_values_offset_buff
                            .write_u32::<E>(field_values_offset_value)
                            .unwrap();
                        if field_offset_needs_padding {
                            wrt.push(0);
                        }
                        for long in longs.iter() {
                            wrt.write_u32::<E>(*long).unwrap();
                        }
                    }
                };
            }
            IfdFieldValues::Rationals(rationals) => {
                field_values_offset_buff
                    .write_u32::<E>(field_values_offset_value)
                    .unwrap();
                if field_offset_needs_padding {
                    wrt.push(0);
                }
                for URational {
                    numerator,
                    denominator,
                } in rationals.iter()
                {
                    wrt.write_u32::<E>(*numerator).unwrap();
                    wrt.write_u32::<E>(*denominator).unwrap();
                }
            }
        }
    }

    const fn field_type_tag(&self) -> IfdFieldType {
        match self {
            IfdFieldValues::Bytes(_) => IfdFieldType::Byte,
            IfdFieldValues::ASCII(_) => IfdFieldType::ASCII,
            IfdFieldValues::Shorts(_) => IfdFieldType::Short,
            IfdFieldValues::Longs(_) => IfdFieldType::Long,
            IfdFieldValues::Rationals(_) => IfdFieldType::Rational,
        }
    }

    fn num_values(&self) -> u32 {
        match self {
            IfdFieldValues::Bytes(bytes) => bytes.len() as u32,
            IfdFieldValues::ASCII(_) => 1,
            IfdFieldValues::Shorts(short) => short.len() as u32,
            IfdFieldValues::Longs(long) => long.len() as u32,
            IfdFieldValues::Rationals(rational) => rational.len() as u32,
        }
    }
}
