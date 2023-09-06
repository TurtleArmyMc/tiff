use std::{marker::PhantomData, slice::ChunksExact};

use crate::{
    colors,
    encode::{encode_ifds, private::EncodeResult},
    ifd,
    types::{Short, URational},
    Image,
};

use super::{
    buffer::TiffEncodeBuffer,
    compression::Compression,
    private::{IfdInfo, ImageEncoderImpl},
    EncodeEndianness, ImageEncoder,
};

pub struct RGBImageEncoder<'a, E, C>
where
    C: Compression<colors::RGB>,
{
    image: &'a Image<colors::RGB>,
    image_compressor: C,
    endianness: PhantomData<E>,
}

impl<'a, E: EncodeEndianness, C: Compression<colors::RGB>> RGBImageEncoder<'a, E, C> {
    pub fn new(image: &'a Image<colors::RGB>, compression: C) -> Self {
        Self {
            image,
            image_compressor: compression,

            endianness: PhantomData,
        }
    }
}

impl<'a, E: EncodeEndianness, C: Compression<colors::RGB>> ImageEncoder
    for RGBImageEncoder<'a, E, C>
{
}

impl<'a, E: EncodeEndianness, C: Compression<colors::RGB>> ImageEncoderImpl
    for RGBImageEncoder<'a, E, C>
{
    type Endianness = E;

    fn append_image_to_buffer(&self, wrt: &mut TiffEncodeBuffer<E>) -> IfdInfo {
        let EncodeResult {
            image_strip_offsets,
            image_strip_bytecounts,
        } = encode_rgb_img(wrt, self.image.rows(), &self.image_compressor);

        let ifd_inx = wrt.align_and_get_len();

        let ifd_entries = [
            ifd::Entry::new(
                ifd::Tag::ImageWidth,
                ifd::Values::Longs(vec![self.image.width().try_into().unwrap()]),
            ),
            ifd::Entry::new(
                ifd::Tag::ImageLength,
                ifd::Values::Longs(vec![self.image.height().try_into().unwrap()]),
            ),
            ifd::Entry::new(ifd::Tag::BitsPerSample, ifd::Values::Shorts(vec![8, 8, 8])),
            ifd::Entry::new(
                ifd::Tag::Compression,
                ifd::Values::Shorts(vec![self.image_compressor.compression_type_tag() as Short]),
            ),
            ifd::Entry::new(
                ifd::Tag::PhotometricInterpretation,
                ifd::Values::Shorts(vec![ifd::tags::PhotometricInterpretation::RGB as Short]),
            ),
            ifd::Entry::new(
                ifd::Tag::StripOffsets,
                ifd::Values::Shorts(image_strip_offsets),
            ),
            ifd::Entry::new(ifd::Tag::SamplesPerPixel, ifd::Values::Shorts(vec![3])),
            ifd::Entry::new(
                ifd::Tag::RowsPerStrip,
                ifd::Values::Shorts(vec![self.image.height().try_into().unwrap()]),
            ),
            ifd::Entry::new(
                ifd::Tag::StripByteCounts,
                ifd::Values::Longs(image_strip_bytecounts),
            ),
            ifd::Entry::new(
                ifd::Tag::XResolution,
                ifd::Values::Rationals(vec![URational::new(1, 1)]),
            ),
            ifd::Entry::new(
                ifd::Tag::YResolution,
                ifd::Values::Rationals(vec![URational::new(1, 1)]),
            ),
        ];
        let entry_count = ifd_entries.len();

        debug_assert!(
            ifd_entries
                .iter()
                .zip(ifd_entries.iter().skip(1))
                .all(|(prev, next)| prev.tag() <= next.tag()),
            "IFD entries are not sorted by tag"
        );

        encode_ifds(wrt, ifd_entries.into_iter());

        IfdInfo {
            inx: ifd_inx,
            entry_count,
        }
    }
}

fn encode_rgb_img<C: Compression<colors::RGB>, E: EncodeEndianness>(
    wrt: &mut TiffEncodeBuffer<E>,
    rows: ChunksExact<'_, colors::RGB>,
    image_compressor: &C,
) -> EncodeResult {
    let row_inx = wrt.align_and_get_len();

    image_compressor.encode(
        wrt,
        rows.flatten().flat_map(|pixel| [pixel.r, pixel.g, pixel.b]),
    );

    EncodeResult {
        image_strip_offsets: vec![row_inx.try_into().unwrap()],
        image_strip_bytecounts: vec![(wrt.len() - row_inx).try_into().unwrap()],
    }
}
