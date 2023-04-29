pub mod bilevel;
mod buffer;
pub mod compression;
mod image_header;

pub use bilevel::BilevelImageEncoder;
pub use image_header::EncodeEndianness;

use crate::{ifd, types::Long};

use self::buffer::TiffEncodeBuffer;

pub trait Encoder: private::EncoderImpl {
    fn encode(&self) -> Vec<u8> {
        let mut encoded = TiffEncodeBuffer::<Self::Endianness>::new();

        let ifd_inx = self.append_to_buffer(&mut encoded).try_into().unwrap();
        // Update header to point to the correct IDF offset
        encoded.get_tiff_header().set_first_ifd_offset(ifd_inx);

        encoded.to_bytes()
    }
}

/// Encodes headers and ifd::values for entries
pub(crate) fn encode_ifds<E: EncodeEndianness, I: ExactSizeIterator<Item = ifd::Entry>>(
    wrt: &mut TiffEncodeBuffer<E>,
    ifds: I,
) {
    let field_count = ifds.len();
    let ifd_inx = wrt.append_new_ifd(field_count);

    for (entry_num, entry) in ifds.enumerate() {
        let value_offset = wrt.append_ifd_value(entry.values());
        wrt.get_ifd_at(ifd_inx, field_count)
            .get_entry(entry_num)
            .set_all(&entry, value_offset);
    }
}

impl ifd::Values {
    pub(crate) const fn field_type_tag(&self) -> ifd::Type {
        match self {
            ifd::Values::Bytes(_) => ifd::Type::Byte,
            ifd::Values::ASCII(_) => ifd::Type::ASCII,
            ifd::Values::Shorts(_) => ifd::Type::Short,
            ifd::Values::Longs(_) => ifd::Type::Long,
            ifd::Values::Rationals(_) => ifd::Type::Rational,
        }
    }

    pub(crate) fn num_values(&self) -> Long {
        match self {
            ifd::Values::Bytes(bytes) => bytes.len().try_into().unwrap(),
            ifd::Values::ASCII(_) => 1,
            ifd::Values::Shorts(short) => short.len().try_into().unwrap(),
            ifd::Values::Longs(long) => long.len().try_into().unwrap(),
            ifd::Values::Rationals(rational) => rational.len().try_into().unwrap(),
        }
    }
}

pub(crate) mod private {
    use crate::types::Short;

    use super::{buffer::TiffEncodeBuffer, EncodeEndianness};

    pub struct EncodeResult {
        pub(crate) image_strip_offsets: Vec<Short>,
        pub(crate) image_strip_bytecounts: Vec<Short>,
    }

    pub trait EncoderImpl {
        type Endianness: EncodeEndianness;

        fn append_to_buffer(&self, wrt: &mut TiffEncodeBuffer<Self::Endianness>) -> usize;
    }
}
