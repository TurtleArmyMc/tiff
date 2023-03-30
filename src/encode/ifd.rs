use crate::{
    ifd::ifd::{IFDEntry, IfdFieldType, IfdFieldValues},
    types::Long,
};

use super::{buffer::TiffEncodeBuffer, EncodeEndianness};

/// Encodes headers and values for entries
pub(crate) fn encode_ifds<E: EncodeEndianness>(
    wrt: &mut TiffEncodeBuffer<E>,
    mut ifds: Vec<IFDEntry>,
) {
    ifds.sort_unstable_by_key(|ifd| ifd.tag());

    let ifd_inx = wrt.append_new_ifd(ifds.len());

    for (entry_num, entry) in ifds.iter().enumerate() {
        let value_offset = wrt.append_ifd_value(entry.values());
        wrt.get_ifd_at(ifd_inx, ifds.len())
            .get_entry(entry_num)
            .set_all(entry, value_offset);
    }
}

impl IfdFieldValues {
    pub(crate) const fn field_type_tag(&self) -> IfdFieldType {
        match self {
            IfdFieldValues::Bytes(_) => IfdFieldType::Byte,
            IfdFieldValues::ASCII(_) => IfdFieldType::ASCII,
            IfdFieldValues::Shorts(_) => IfdFieldType::Short,
            IfdFieldValues::Longs(_) => IfdFieldType::Long,
            IfdFieldValues::Rationals(_) => IfdFieldType::Rational,
        }
    }

    pub(crate) fn num_values(&self) -> Long {
        match self {
            IfdFieldValues::Bytes(bytes) => bytes.len().try_into().unwrap(),
            IfdFieldValues::ASCII(_) => 1,
            IfdFieldValues::Shorts(short) => short.len().try_into().unwrap(),
            IfdFieldValues::Longs(long) => long.len().try_into().unwrap(),
            IfdFieldValues::Rationals(rational) => rational.len().try_into().unwrap(),
        }
    }
}
