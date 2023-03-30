mod ifd;
mod image_header;
use byteorder::WriteBytesExt;
pub use image_header::EncodeEndianness;
use itertools::Itertools;

use crate::{
    colors,
    ifd::{
        ifd::{IFDEntry, IfdFieldTag, IfdFieldValues, URational},
        tags,
    },
    Image,
};

use self::{ifd::encode_ifds, image_header::encode_header};

pub fn encode<E>(image: &Image<colors::Bilevel>) -> Vec<u8>
where
    E: EncodeEndianness,
{
    let mut encoded = Vec::new();
    encode_header::<E>(&mut encoded);

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
        encoded.push(packed_pixels);
    }

    // Update header to point to the correct IDF offset
    if encoded.len() % 2 == 1 {
        // Make sure that header offset is on a word boundry
        encoded.push(0);
    }
    let offset = encoded.len() as u32;
    (&mut encoded[4..8]).write_u32::<E>(offset).unwrap();

    let mut entries: Vec<IFDEntry> = Vec::new();

    entries.push(IFDEntry::new(
        IfdFieldTag::ImageWidth,
        IfdFieldValues::Shorts(Vec::from_iter([image.width() as u16])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::ImageLength,
        IfdFieldValues::Shorts(Vec::from_iter([image.height() as u16])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::Compression,
        IfdFieldValues::Shorts(Vec::from_iter([tags::Compression::NoCompression as u16])),
    ));
    entries.push(IFDEntry::new(
        IfdFieldTag::PhotometricInterpretation,
        IfdFieldValues::Shorts(Vec::from_iter([
            tags::PhotometricInterpretation::BlackIsZero as u16,
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

    encoded
}
