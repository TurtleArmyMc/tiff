use std::marker::PhantomData;

use crate::{
    colors,
    encode::{encode_ifds, private::EncodeResult},
    ifd,
    types::{Short, URational},
    Image,
};

use super::{
    buffer::TiffEncodeBuffer,
    private::{IfdInfo, ImageEncoderImpl},
    EncodeEndianness, ImageEncoder,
};

pub trait Compression: private::ImageWriter {}

#[derive(Clone, Copy)]
pub struct NoCompression;
impl Compression for NoCompression {}

pub struct PaletteColorImageEncoder<'a, E, C>
where
    C: Compression,
{
    image: &'a Image<colors::PaletteColor<'a>>,
    image_compressor: C,
    endianness: PhantomData<E>,
}

impl<'a, E: EncodeEndianness, C: Compression> PaletteColorImageEncoder<'a, E, C> {
    pub fn new(image: &'a Image<colors::PaletteColor<'a>>, compression: C) -> Self {
        Self {
            image,
            image_compressor: compression,
            endianness: PhantomData,
        }
    }
}

impl<'a, E: EncodeEndianness, C: Compression> ImageEncoder for PaletteColorImageEncoder<'a, E, C> {}

impl<'a, E: EncodeEndianness, C: Compression> ImageEncoderImpl
    for PaletteColorImageEncoder<'a, E, C>
{
    type Endianness = E;

    fn append_image_to_buffer(&self, wrt: &mut TiffEncodeBuffer<E>) -> IfdInfo {
        let EncodeResult {
            image_strip_offsets,
            image_strip_bytecounts,
        } = self
            .image_compressor
            .encode_palettized_img(wrt, self.image.pixels());

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
            // TODO: Automatically switch to 4bit when possible
            ifd::Entry::new(ifd::Tag::BitsPerSample, ifd::Values::Shorts(vec![8])),
            ifd::Entry::new(
                ifd::Tag::Compression,
                ifd::Values::Shorts(vec![self.image_compressor.compression_type_tag() as Short]),
            ),
            ifd::Entry::new(
                ifd::Tag::PhotometricInterpretation,
                ifd::Values::Shorts(vec![
                    ifd::tags::PhotometricInterpretation::PaletteColor as Short,
                ]),
            ),
            ifd::Entry::new(
                ifd::Tag::StripOffsets,
                ifd::Values::Shorts(image_strip_offsets),
            ),
            ifd::Entry::new(
                ifd::Tag::RowsPerStrip,
                ifd::Values::Shorts(vec![self.image.height().try_into().unwrap()]),
            ),
            ifd::Entry::new(
                ifd::Tag::StripByteCounts,
                ifd::Values::Shorts(image_strip_bytecounts),
            ),
            ifd::Entry::new(
                ifd::Tag::XResolution,
                ifd::Values::Rationals(vec![URational::new(1, 1)]),
            ),
            ifd::Entry::new(
                ifd::Tag::YResolution,
                ifd::Values::Rationals(vec![URational::new(1, 1)]),
            ),
            ifd::Entry::new(
                ifd::Tag::ColorMap,
                ifd::Values::Shorts(self.image.get_colormap().create_colormap_vec()),
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

pub(crate) mod private {
    use super::NoCompression;
    use crate::encode::private::EncodeResult;
    use crate::{
        colors,
        encode::{buffer::TiffEncodeBuffer, EncodeEndianness},
        ifd,
    };
    use std::slice::ChunksExact;

    pub trait ImageWriter: Copy {
        fn compression_type_tag(&self) -> ifd::tags::Compression;

        fn encode_palettized_img<E: EncodeEndianness>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            pixels: ChunksExact<'_, colors::PaletteColor>,
        ) -> EncodeResult;
    }

    impl ImageWriter for NoCompression {
        fn compression_type_tag(&self) -> ifd::tags::Compression {
            ifd::tags::Compression::NoCompression
        }

        fn encode_palettized_img<E: EncodeEndianness>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            pixels: ChunksExact<'_, colors::PaletteColor>,
        ) -> EncodeResult {
            let row_inx = wrt.align_and_get_len().try_into().unwrap();
            let mut byte_count = 0;

            wrt.extend_bytes(pixels.flatten().map(|pixel| {
                byte_count += 1;
                pixel.get_inx()
            }));

            EncodeResult {
                image_strip_offsets: vec![row_inx],
                image_strip_bytecounts: vec![byte_count],
            }
        }
    }
}
