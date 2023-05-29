pub mod bilevel;
mod buffer;
mod compression;
pub mod grayscale_4bit;
pub mod grayscale_8bit;
mod image_header;
pub mod palette_color;
pub mod rgb;

pub use bilevel::BilevelImageEncoder;
pub use image_header::EncodeEndianness;

use crate::{ifd, types::Long};

use self::buffer::TiffEncodeBuffer;

/// Encodes multiple images into a single file.
///
/// # Panics
/// Panics if the iterator has no elements.
pub fn encode_images<'a, Endianness, E, I>(mut images: I) -> Vec<u8>
where
    Endianness: EncodeEndianness + 'static,
    E: ImageEncoder<Endianness = Endianness> + ?Sized + 'a,
    I: Iterator<Item = &'a E>,
{
    let mut encoded = TiffEncodeBuffer::<Endianness>::new();

    let mut prev_ifd_info = match images.next() {
        Some(first) => {
            let ifd_info = first.append_image_to_buffer(&mut encoded);
            encoded
                .get_tiff_header()
                .set_first_ifd_offset(ifd_info.inx.try_into().unwrap());
            ifd_info
        }
        None => panic!("tiff file must have at least one image"),
    };
    for image in images {
        let ifd_info = image.append_image_to_buffer(&mut encoded);
        encoded
            .get_ifd_at(prev_ifd_info.inx, prev_ifd_info.entry_count)
            .set_next_ifd_offset(ifd_info.inx.try_into().unwrap());
        prev_ifd_info = ifd_info;
    }

    encoded.to_bytes()
}

pub trait ImageEncoder: private::ImageEncoderImpl {
    /// Encodes image into a file with that single image.
    fn encode(&self) -> Vec<u8> {
        let mut encoded = TiffEncodeBuffer::<Self::Endianness>::new();

        let ifd_inx = self
            .append_image_to_buffer(&mut encoded)
            .inx
            .try_into()
            .unwrap();
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
    use crate::types::{Long, Short};

    use super::{buffer::TiffEncodeBuffer, EncodeEndianness};

    pub struct IfdInfo {
        pub(crate) inx: usize,
        pub(crate) entry_count: usize,
    }

    pub struct EncodeResult {
        pub(crate) image_strip_offsets: Vec<Short>,
        pub(crate) image_strip_bytecounts: Vec<Long>,
    }

    pub trait ImageEncoderImpl {
        type Endianness: EncodeEndianness;

        /// Appends an image to a buffer and returns the index and number of entries in its image field directory.
        /// This may or may not be the only image in the file.
        fn append_image_to_buffer(&self, wrt: &mut TiffEncodeBuffer<Self::Endianness>) -> IfdInfo;
    }
}
