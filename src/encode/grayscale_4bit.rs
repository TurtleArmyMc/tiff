use std::{marker::PhantomData, slice::ChunksExact};

use crate::{
    colors,
    encode::{encode_ifds, private::EncodeResult},
    ifd,
    types::{Byte, Short, URational},
    Image,
};

use super::{
    buffer::TiffEncodeBuffer,
    compression::{Compression, HalfBytePacker},
    photo_interp::{self, PhotometricInterpretation},
    private::{IfdInfo, ImageEncoderImpl},
    EncodeEndianness, ImageEncoder,
};

pub struct Grayscale4BitImageEncoder<'a, E, C, P = photo_interp::BlackIsZero>
where
    C: Compression<colors::Grayscale4Bit>,
    P: PhotometricInterpretation<colors::Grayscale4Bit>,
{
    image: &'a Image<colors::Grayscale4Bit>,
    image_compressor: C,
    photo_interp: P,
    endianness: PhantomData<E>,
}

impl<'a, E, C, P> Grayscale4BitImageEncoder<'a, E, C, P>
where
    E: EncodeEndianness,
    C: Compression<colors::Grayscale4Bit>,
    P: PhotometricInterpretation<colors::Grayscale4Bit>,
{
    pub fn new(image: &'a Image<colors::Grayscale4Bit>, compression: C, photo_interp: P) -> Self {
        Self {
            image,
            image_compressor: compression,
            photo_interp,
            endianness: PhantomData,
        }
    }
}

impl<'a, E, C, P> ImageEncoder for Grayscale4BitImageEncoder<'a, E, C, P>
where
    E: EncodeEndianness,
    C: Compression<colors::Grayscale4Bit>,
    P: PhotometricInterpretation<colors::Grayscale4Bit>,
{
}

impl<'a, E, C, P> ImageEncoderImpl for Grayscale4BitImageEncoder<'a, E, C, P>
where
    E: EncodeEndianness,
    C: Compression<colors::Grayscale4Bit>,
    P: PhotometricInterpretation<colors::Grayscale4Bit>,
{
    type Endianness = E;

    fn append_image_to_buffer(&self, wrt: &mut TiffEncodeBuffer<E>) -> IfdInfo {
        let EncodeResult {
            image_strip_offsets,
            image_strip_bytecounts,
        } = encode_grayscale_img(
            wrt,
            self.image.pixels(),
            &self.photo_interp,
            &self.image_compressor,
        );

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
            ifd::Entry::new(ifd::Tag::BitsPerSample, ifd::Values::Shorts(vec![4])),
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

fn encode_grayscale_img<E, C, P>(
    wrt: &mut TiffEncodeBuffer<E>,
    pixels: ChunksExact<'_, colors::Grayscale4Bit>,
    photo_iterp: &P,
    image_compressor: &C,
) -> EncodeResult
where
    E: EncodeEndianness,
    C: Compression<colors::Grayscale4Bit>,
    P: PhotometricInterpretation<colors::Grayscale4Bit>,
{
    let row_inx = wrt.align_and_get_len();

    image_compressor.encode(
        wrt,
        pixels.flat_map(|row| {
            HalfBytePacker::new(row.iter().map(|pixel| photo_iterp.encode_pixel(*pixel)))
        }),
    );

    EncodeResult {
        image_strip_offsets: vec![row_inx.try_into().unwrap()],
        image_strip_bytecounts: vec![(wrt.len() - row_inx).try_into().unwrap()],
    }
}

impl photo_interp::private::PhotometricInterpretationImpl<colors::Grayscale4Bit>
    for photo_interp::BlackIsZero
{
    fn encode_pixel(&self, pixel: colors::Grayscale4Bit) -> Byte {
        pixel.value()
    }
}

impl photo_interp::private::PhotometricInterpretationImpl<colors::Grayscale4Bit>
    for photo_interp::WhiteIsZero
{
    fn encode_pixel(&self, pixel: colors::Grayscale4Bit) -> Byte {
        0b1111 - pixel.value()
    }
}
