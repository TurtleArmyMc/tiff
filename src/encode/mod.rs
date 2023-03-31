mod buffer;
mod image_header;
pub use image_header::EncodeEndianness;
use itertools::Itertools;

use crate::{
    colors, ifd,
    types::{Long, Short, URational},
    Image,
};

use self::buffer::TiffEncodeBuffer;

pub fn encode<E>(image: &Image<colors::Bilevel>) -> Vec<u8>
where
    E: EncodeEndianness,
{
    let mut encoded = TiffEncodeBuffer::<E>::new();

    let row_byte_count = image.width() / 8 + (image.width() % 8).min(1);
    let byte_count = row_byte_count * image.height();

    for row in image.pixels() {
        for mut eight_pixels in &row.iter().chunks(8) {
            let mut packed_pixels = 0;
            for bit in (0..8).rev() {
                packed_pixels |= eight_pixels
                    .next()
                    .map(|pixel| match pixel {
                        colors::Bilevel::Black => 0,
                        colors::Bilevel::White => 1,
                    })
                    .unwrap_or(0)
                    << bit;
            }
            encoded.append_byte(packed_pixels);
        }
    }

    // Update header to point to the correct IDF offset
    let ifd_inx = encoded.align_and_get_len().try_into().unwrap();
    encoded.get_tiff_header().set_first_ifd_offset(ifd_inx);

    let mut entries: Vec<ifd::Entry> = Vec::new();

    entries.push(ifd::Entry::new(
        ifd::Tag::ImageWidth,
        ifd::Values::Longs(Vec::from_iter([image.width().try_into().unwrap()])),
    ));
    entries.push(ifd::Entry::new(
        ifd::Tag::ImageLength,
        ifd::Values::Longs(Vec::from_iter([image.height().try_into().unwrap()])),
    ));
    entries.push(ifd::Entry::new(
        ifd::Tag::Compression,
        ifd::Values::Shorts(Vec::from_iter([
            ifd::tags::Compression::NoCompression as Short
        ])),
    ));
    entries.push(ifd::Entry::new(
        ifd::Tag::PhotometricInterpretation,
        ifd::Values::Shorts(Vec::from_iter([
            ifd::tags::PhotometricInterpretation::BlackIsZero as Short,
        ])),
    ));

    entries.push(ifd::Entry::new(
        ifd::Tag::StripOffsets,
        // The only strip starts immediately after the header
        ifd::Values::Shorts(Vec::from_iter([8])),
    ));
    entries.push(ifd::Entry::new(
        ifd::Tag::RowsPerStrip,
        ifd::Values::Shorts(Vec::from_iter([image
            .height()
            .try_into()
            .expect("too many rows in image")])),
    ));
    entries.push(ifd::Entry::new(
        ifd::Tag::StripByteCounts,
        ifd::Values::Shorts(Vec::from_iter([byte_count
            .try_into()
            .expect("too many pixels in the image")])),
    ));

    entries.push(ifd::Entry::new(
        ifd::Tag::XResolution,
        ifd::Values::Rationals(Vec::from_iter([URational::new(1, 1)])),
    ));
    entries.push(ifd::Entry::new(
        ifd::Tag::YResolution,
        ifd::Values::Rationals(Vec::from_iter([URational::new(1, 1)])),
    ));

    encode_ifds::<E>(&mut encoded, entries);

    encoded.to_bytes()
}

/// Encodes headers and ifd::values for entries
pub(crate) fn encode_ifds<E: EncodeEndianness>(
    wrt: &mut TiffEncodeBuffer<E>,
    mut ifds: Vec<ifd::Entry>,
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
