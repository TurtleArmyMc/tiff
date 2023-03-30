mod buffer;
mod ifd;
mod image_header;
pub use image_header::EncodeEndianness;
use itertools::Itertools;

use crate::{
    colors,
    ifd::{
        ifd::{IFDEntry, IfdFieldTag, IfdFieldValues},
        tags,
    },
    types::{Short, URational},
    Image,
};

use self::{buffer::TiffEncodeBuffer, ifd::encode_ifds};

pub fn encode<E>(image: &Image<colors::Bilevel>) -> Vec<u8>
where
    E: EncodeEndianness,
{
    let mut encoded = TiffEncodeBuffer::<E>::new();

    let byte_count = image.pixel_count() / 8 + if image.pixel_count() % 8 != 0 { 1 } else { 0 };
    for mut eight_pixels in &image.pixels().flatten().chunks(8) {
        let mut packed_pixels = 0;
        for _ in 0..8 {
            let pixel = eight_pixels.next().unwrap_or(&colors::Bilevel::Black);
            packed_pixels = (packed_pixels << 1)
                | match pixel {
                    colors::Bilevel::Black => 0,
                    colors::Bilevel::White => 1,
                }
        }
        encoded.append_byte(packed_pixels);
    }

    // Update header to point to the correct IDF offset
    let ifd_inx = encoded.align_and_get_len().try_into().unwrap();
    encoded.get_tiff_header().set_first_ifd_offset(ifd_inx);

    let mut entries: Vec<IFDEntry> = Vec::new();

    entries.push(IFDEntry::new(
        IfdFieldTag::ImageWidth,
        IfdFieldValues::Shorts(Vec::from_iter([image.width().try_into().unwrap()])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::ImageLength,
        IfdFieldValues::Shorts(Vec::from_iter([image.height().try_into().unwrap()])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::Compression,
        IfdFieldValues::Shorts(Vec::from_iter([tags::Compression::NoCompression as Short])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::PhotometricInterpretation,
        IfdFieldValues::Shorts(Vec::from_iter([
            tags::PhotometricInterpretation::BlackIsZero as Short,
        ])),
    ));

    entries.push(IFDEntry::new(
        IfdFieldTag::StripOffsets,
        // The only strip starts immediately after the header
        IfdFieldValues::Shorts(Vec::from_iter([8])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::RowsPerStrip,
        IfdFieldValues::Shorts(Vec::from_iter([image
            .height()
            .try_into()
            .expect("too many rows in image")])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::StripByteCounts,
        IfdFieldValues::Shorts(Vec::from_iter([byte_count
            .try_into()
            .expect("too many pixels in the image")])),
    ));

    entries.push(IFDEntry::new(
        IfdFieldTag::XResolution,
        IfdFieldValues::Rationals(Vec::from_iter([URational::new(1, 1)])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::YResolution,
        IfdFieldValues::Rationals(Vec::from_iter([URational::new(1, 1)])),
    ));

    encode_ifds::<E>(&mut encoded, entries);

    encoded.to_bytes()
}
