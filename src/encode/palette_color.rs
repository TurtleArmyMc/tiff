use std::{marker::PhantomData, slice::ChunksExact};

use crate::{
    colors,
    compression::Compression,
    encode::{encode_ifds, private::EncodeResult},
    ifd,
    types::{Short, URational},
    Image,
};

use super::{
    buffer::TiffEncodeBuffer,
    compression::HalfBytePacker,
    private::{IfdInfo, ImageEncoderImpl},
    EncodeEndianness, ImageEncoder,
};

pub struct PaletteColorImageEncoder<'a, E, C>
where
    C: Compression<colors::PaletteColor<'a>>,
{
    image: &'a Image<colors::PaletteColor<'a>>,
    image_compressor: C,
    endianness: PhantomData<E>,
}

impl<'a, E: EncodeEndianness, C: Compression<colors::PaletteColor<'a>>>
    PaletteColorImageEncoder<'a, E, C>
{
    pub fn new(image: &'a Image<colors::PaletteColor<'a>>, compression: C) -> Self {
        Self {
            image,
            image_compressor: compression,
            endianness: PhantomData,
        }
    }
}

impl<'a, E, C> ImageEncoder for PaletteColorImageEncoder<'a, E, C>
where
    E: EncodeEndianness,
    C: Compression<colors::PaletteColor<'a>>,
{
}

impl<'a, E, C> ImageEncoderImpl for PaletteColorImageEncoder<'a, E, C>
where
    E: EncodeEndianness,
    C: Compression<colors::PaletteColor<'a>>,
{
    type Endianness = E;

    fn append_image_to_buffer(&self, wrt: &mut TiffEncodeBuffer<E>) -> IfdInfo {
        let EncodeResult {
            image_strip_offsets,
            image_strip_bytecounts,
        } = encode_palettized_img(
            wrt,
            self.image.rows(),
            &self.image_compressor,
            self.image.bits_per_palette_sample() as u8,
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
            ifd::Entry::new(
                ifd::Tag::BitsPerSample,
                ifd::Values::Shorts(vec![self.image.bits_per_palette_sample()]),
            ),
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

fn encode_palettized_img<'a, C, E>(
    wrt: &mut TiffEncodeBuffer<E>,
    rows: ChunksExact<'_, colors::PaletteColor>,
    image_compressor: &C,
    bits_per_sample: u8,
) -> EncodeResult
where
    C: Compression<colors::PaletteColor<'a>>,
    E: EncodeEndianness,
{
    let row_inx = wrt.align_and_get_len();

    if bits_per_sample == 8 {
        image_compressor.encode(wrt, rows.flatten().map(colors::PaletteColor::get_inx));
    } else {
        // 4 bits per sample
        image_compressor.encode(
            wrt,
            rows.flat_map(|row| HalfBytePacker::new(row.iter().map(colors::PaletteColor::get_inx))),
        );
    }

    EncodeResult {
        image_strip_offsets: vec![row_inx.try_into().unwrap()],
        image_strip_bytecounts: vec![(wrt.len() - row_inx).try_into().unwrap()],
    }
}
