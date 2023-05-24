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

pub trait PhotometricInterpretation: private::PhotometricInterpretationImpl {}
#[derive(Clone, Copy)]
pub struct BlackIsZero;
#[derive(Clone, Copy)]
pub struct WhiteIsZero;

impl PhotometricInterpretation for BlackIsZero {}
impl PhotometricInterpretation for WhiteIsZero {}

pub trait Compression: private::ImageWriter {}

#[derive(Clone, Copy)]
pub struct NoCompression;
impl Compression for NoCompression {}

pub struct BilevelImageEncoder<'a, E, C, P = BlackIsZero>
where
    C: Compression,
    P: PhotometricInterpretation,
{
    image: &'a Image<colors::Bilevel>,
    image_compressor: C,
    photo_interp: P,
    endianness: PhantomData<E>,
}

impl<'a, E: EncodeEndianness, C: Compression, P: PhotometricInterpretation>
    BilevelImageEncoder<'a, E, C, P>
{
    pub fn new(image: &'a Image<colors::Bilevel>, compression: C, photo_interp: P) -> Self {
        Self {
            image,
            image_compressor: compression,
            photo_interp,
            endianness: PhantomData,
        }
    }
}

impl<'a, E: EncodeEndianness, C: Compression, P: PhotometricInterpretation> ImageEncoder
    for BilevelImageEncoder<'a, E, C, P>
{
}

impl<'a, E: EncodeEndianness, C: Compression, P: PhotometricInterpretation> ImageEncoderImpl
    for BilevelImageEncoder<'a, E, C, P>
{
    type Endianness = E;

    fn append_image_to_buffer(&self, wrt: &mut TiffEncodeBuffer<E>) -> IfdInfo {
        let EncodeResult {
            image_strip_offsets,
            image_strip_bytecounts,
        } = self
            .image_compressor
            .encode_bilevel_img(wrt, self.image.pixels(), self.photo_interp);

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
            ifd::Entry::new(
                ifd::Tag::Compression,
                ifd::Values::Shorts(vec![self.image_compressor.compression_type_tag() as Short]),
            ),
            ifd::Entry::new(
                ifd::Tag::PhotometricInterpretation,
                ifd::Values::Shorts(vec![self.photo_interp.tag() as Short]),
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

pub(crate) mod private {
    use super::{BlackIsZero, NoCompression, WhiteIsZero};
    use crate::encode::private::EncodeResult;
    use crate::{
        colors,
        encode::{bilevel::PhotometricInterpretation, buffer::TiffEncodeBuffer, EncodeEndianness},
        ifd,
    };
    use itertools::Itertools;
    use std::slice::ChunksExact;

    pub trait PhotometricInterpretationImpl: Copy {
        fn encode_pixel(&self, pixel: colors::Bilevel) -> u8;
        fn tag(&self) -> ifd::tags::PhotometricInterpretation;
    }

    impl PhotometricInterpretationImpl for BlackIsZero {
        fn encode_pixel(&self, pixel: colors::Bilevel) -> u8 {
            match pixel {
                colors::Bilevel::Black => 0,
                colors::Bilevel::White => 1,
            }
        }

        fn tag(&self) -> ifd::tags::PhotometricInterpretation {
            ifd::tags::PhotometricInterpretation::BlackIsZero
        }
    }

    impl PhotometricInterpretationImpl for WhiteIsZero {
        fn encode_pixel(&self, pixel: colors::Bilevel) -> u8 {
            match pixel {
                colors::Bilevel::Black => 1,
                colors::Bilevel::White => 0,
            }
        }

        fn tag(&self) -> ifd::tags::PhotometricInterpretation {
            ifd::tags::PhotometricInterpretation::WhiteIsZero
        }
    }

    pub trait ImageWriter: Copy {
        fn compression_type_tag(&self) -> ifd::tags::Compression;

        fn encode_bilevel_img<E: EncodeEndianness, P: PhotometricInterpretation>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            pixels: ChunksExact<'_, colors::Bilevel>,
            photo_iterp: P,
        ) -> EncodeResult;
    }

    impl ImageWriter for NoCompression {
        fn compression_type_tag(&self) -> ifd::tags::Compression {
            ifd::tags::Compression::NoCompression
        }

        fn encode_bilevel_img<E: EncodeEndianness, P: PhotometricInterpretation>(
            &self,
            wrt: &mut TiffEncodeBuffer<E>,
            pixels: ChunksExact<'_, colors::Bilevel>,
            photo_iterp: P,
        ) -> EncodeResult {
            let row_inx = wrt.align_and_get_len().try_into().unwrap();
            let mut byte_count = 0;

            for row in pixels {
                for eight_pixels in &row.iter().chunks(8) {
                    let mut packed_pixels = 0;
                    for (bit, pixel) in (0..8).rev().zip(eight_pixels) {
                        packed_pixels |= photo_iterp.encode_pixel(*pixel) << bit;
                    }
                    byte_count += 1;
                    wrt.append_byte(packed_pixels);
                }
            }

            EncodeResult {
                image_strip_offsets: vec![row_inx],
                image_strip_bytecounts: vec![byte_count],
            }
        }
    }
}
