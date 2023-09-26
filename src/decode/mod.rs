mod bilevel;
pub mod compression;
mod grayscale_4bit;
mod grayscale_8bit;
mod palette_color;
mod rgb;
mod tiff_field;

use std::{
    borrow::BorrowMut,
    io::{self, BufRead},
    slice,
};

use byteordered::{Endian, Endianness};

use crate::{colors, ifd, types::URational, Image};

use self::compression::{sealed::DecompressionImpl, tag_to_decompressor};

#[allow(unused)]
pub fn decode_images(bytes: &[u8]) -> Result<DecodeResult, DecodeError> {
    // Read endianness from header and check magic number
    let endianness = match bytes {
        // II
        [73, 73, 42, 00, ..] => Endianness::Little,
        // MM
        [77, 77, 00, 42, ..] => Endianness::Big,
        _ => return Err(DecodeError::InvalidFiletype),
    };

    let mut ifd_index = bytes
        .get(4..8)
        .ok_or(DecodeError::InvalidFiletype)
        .and_then(|slice| {
            endianness
                .read_u32(slice)
                .map_err(|_| DecodeError::InvalidFiletype)
        })? as usize;

    let mut images = Vec::new();
    let mut errors = Vec::new();

    let mut ifd_indices = Vec::new();
    // The final IFD will have an offset of 0 for the next IFD
    while ifd_index != 0 {
        // Make sure we don't get stuck in an infinite loop if an IFD lists a
        // previous IFD as the next one
        if ifd_indices.contains(&ifd_index) {
            // TODO: Include any any successfully decoded images in return
            return Err(DecodeError::LoopingIfdIndices);
        }
        ifd_indices.push(ifd_index);

        // TODO: Include any any successfully decoded images in return
        let ifd = Ifd::new(bytes, ifd_index, endianness)?;
        ifd_index = ifd.next_ifd_index;

        match decode_image(bytes, endianness, ifd) {
            Ok((image, field_errors)) => {
                images.push(image);
                errors.extend(field_errors)
            }
            Err(err) => errors.push(err),
        }
    }

    Ok(DecodeResult { images, errors })
}

pub struct DecodeResult {
    pub images: Vec<DecodedImage>,
    pub errors: Vec<DecodeError>,
}

pub enum DecodedImage {
    BilevelImage(Image<colors::Bilevel>),
    Grayscale8BitImage(Image<colors::Grayscale8Bit>),
    Grayscale4BitImage(Image<colors::Grayscale4Bit>),
    RGBImage(Image<colors::RGB>),
}

#[derive(Debug)]
pub enum DecodeError {
    InvalidFiletype,
    UnknownFieldTag(u16),
    UnknownFieldType(u16),
    CantReadField,
    MissingRequiredField,
    // MissingRequiredField { tag: ifd::Tag, ifd_index: usize },
    LoopingIfdIndices,
    TiffFieldError,
    InvalidImageFieldDirectory(usize),
    UnsupportedCompressionType,
    // UnsupportedCompressionType(ifd::tags::Compression),
    CantReadImage,
    // InvalidTypeForTag {
    //     tag: ifd::Tag,
    //     value_type: ifd::Type,
    // },
    // InvalidTagValueCount {
    //     tag: ifd::Tag,
    //     n: usize,
    // },
    // InvalidTagValues {
    //     tag: ifd::Tag,
    //     values: ifd::Values,
    // },
}

// TODO: Include resolution unit of image
fn decode_image(
    bytes: &[u8],
    endianness: Endianness,
    ifd: Ifd<'_>,
) -> Result<(DecodedImage, Vec<DecodeError>), DecodeError> {
    let (mut fields, field_errors) = read_image_field_directory(bytes, endianness, ifd)?;

    let width = tiff_field::find_required(&fields, ifd::Tag::ImageWidth)
        .and_then(tiff_field::read_single)?;
    let height = tiff_field::find_required(&fields, ifd::Tag::ImageLength)
        .and_then(tiff_field::read_single)?;
    let compression_tag = match tiff_field::find(&fields, ifd::Tag::Compression) {
        Some(values) => tiff_field::read_single_short(values).and_then(|compression| {
            // TODO: Use DecodeError::InvalidTagValues
            ifd::tags::Compression::from_repr(compression).ok_or(DecodeError::TiffFieldError)
        })?,
        None => Default::default(),
    };
    let decompressor = tag_to_decompressor(compression_tag)?;
    let photo_interp = tiff_field::find_required(&fields, ifd::Tag::PhotometricInterpretation)
        .and_then(tiff_field::read_single_short)
        .and_then(|photo_interp| {
            // TODO: Use DecodeError::InvalidTagValues
            ifd::tags::PhotometricInterpretation::from_repr(photo_interp)
                .ok_or(DecodeError::TiffFieldError)
        })?;
    let strip_offsets = tiff_field::take_required(&mut fields, ifd::Tag::StripOffsets)
        .and_then(tiff_field::as_usizes)?;
    let rows_per_strip = tiff_field::find(&fields, ifd::Tag::RowsPerStrip)
        .map(tiff_field::read_single)
        .unwrap_or(Ok(u32::MAX))? as usize;
    let strip_byte_counts = tiff_field::take_required(&mut fields, ifd::Tag::StripByteCounts)
        .and_then(tiff_field::as_usizes)?;
    let x_resolution = tiff_field::find_required(&fields, ifd::Tag::XResolution)
        .and_then(tiff_field::read_single_rational)?;
    let y_resolution = tiff_field::find_required(&fields, ifd::Tag::YResolution)
        .and_then(tiff_field::read_single_rational)?;
    let resolution_unit = match tiff_field::find(&fields, ifd::Tag::ResolutionUnit) {
        Some(values) => tiff_field::read_single_short(values).and_then(|resolution_unit| {
            // TODO: Use DecodeError::InvalidTagValues
            ifd::tags::ResolutionUnit::from_repr(resolution_unit).ok_or(DecodeError::TiffFieldError)
        })?,
        None => Default::default(),
    };

    let info = ImageInfo {
        width: width.try_into().unwrap(),
        height: height.try_into().unwrap(),
        decompressor,
        strip_offsets,
        rows_per_strip,
        strip_byte_counts,
        x_resolution,
        y_resolution,
        resolution_unit,
    };

    let image = match photo_interp {
        ifd::tags::PhotometricInterpretation::WhiteIsZero
        | ifd::tags::PhotometricInterpretation::BlackIsZero => {
            decode_bw_image(bytes, fields, info, photo_interp)
        }
        ifd::tags::PhotometricInterpretation::RGB => {
            rgb::decode_image(fields, info).map(DecodedImage::RGBImage)
        }
        ifd::tags::PhotometricInterpretation::PaletteColor => {
            palette_color::decode_image(fields, info).map(DecodedImage::RGBImage)
        }
    }?;

    Ok((image, field_errors))
}

pub(crate) struct ImageInfo {
    width: usize,
    height: usize,
    decompressor: Box<dyn DecompressionImpl>,
    strip_offsets: Vec<usize>,
    rows_per_strip: usize,
    strip_byte_counts: Vec<usize>,
    x_resolution: URational,
    y_resolution: URational,
    resolution_unit: ifd::tags::ResolutionUnit,
}

#[derive(Clone, Copy)]
struct Ifd<'a> {
    ifd_bytes: &'a [u8],
    index: usize,
    next_ifd_index: usize,
}
impl<'a> Ifd<'a> {
    fn new(bytes: &'a [u8], ifd_index: usize, endianness: Endianness) -> Result<Self, DecodeError> {
        let entry_count = bytes
            .get(ifd_index..ifd_index + ifd::ENTRY_COUNT_LEN)
            .ok_or(DecodeError::InvalidImageFieldDirectory(ifd_index))
            .and_then(|slice| {
                endianness
                    .read_u16(slice)
                    .map_err(|_| DecodeError::InvalidImageFieldDirectory(ifd_index))
            })? as usize;
        let entries_start_inx = ifd_index + ifd::ENTRY_COUNT_LEN;
        let entries_end_inx = entries_start_inx + entry_count * ifd::Entry::LEN;
        let entries_range = entries_start_inx..entries_end_inx;
        let ifd_bytes = &bytes[entries_range];
        let next_ifd_index = endianness
            .read_u32(&bytes[entries_end_inx..entries_end_inx + 4])
            .map_err(|_| DecodeError::InvalidImageFieldDirectory(ifd_index))?
            .try_into()
            .unwrap();
        Ok(Self {
            ifd_bytes,
            index: ifd_index,
            next_ifd_index,
        })
    }

    fn fields(&self) -> slice::ChunksExact<'_, u8> {
        self.ifd_bytes.chunks_exact(ifd::Entry::LEN)
    }
}

fn decode_bw_image(
    bytes: &[u8],
    fields: Vec<ifd::Entry>,
    info: ImageInfo,
    photo_interp: ifd::tags::PhotometricInterpretation,
) -> Result<DecodedImage, DecodeError> {
    let bits_per_sample = match fields.iter().find_map(|field| match field.tag() {
        ifd::Tag::BitsPerSample => Some(field.values()),
        _ => None,
    }) {
        Some(values) => tiff_field::read_single_short(values)?,
        None => 1,
    };
    match bits_per_sample {
        1 => bilevel::decode_image(
            fields,
            info,
            photo_interp == ifd::tags::PhotometricInterpretation::WhiteIsZero,
        )
        .map(DecodedImage::BilevelImage),
        4 => grayscale_4bit::decode_image(
            fields,
            info,
            photo_interp == ifd::tags::PhotometricInterpretation::WhiteIsZero,
        )
        .map(DecodedImage::Grayscale4BitImage),
        8 => grayscale_8bit::decode_image(
            bytes,
            fields,
            info,
            photo_interp == ifd::tags::PhotometricInterpretation::WhiteIsZero,
        )
        .map(DecodedImage::Grayscale8BitImage),
        // TODO: Use DecodeError::InvalidTagValues
        _ => Err(DecodeError::TiffFieldError),
    }
}

fn read_image_field_directory(
    bytes: &[u8],
    endianness: Endianness,
    ifd: Ifd<'_>,
) -> Result<(Vec<ifd::Entry>, Vec<DecodeError>), DecodeError> {
    let entries = ifd.fields().map(|field| -> Result<_, DecodeError> {
        let mut field = io::Cursor::new(field);
        let raw_tag = endianness.read_u16(field.borrow_mut()).unwrap();
        let tag: ifd::Tag =
            ifd::Tag::from_repr(raw_tag).ok_or(DecodeError::UnknownFieldTag(raw_tag))?;
        let raw_field_type = endianness.read_u16(field.borrow_mut()).unwrap();
        let field_type = ifd::Type::from_repr(raw_field_type)
            .ok_or(DecodeError::UnknownFieldType(raw_field_type))?;
        let val_count = endianness.read_u32(field.borrow_mut()).unwrap() as usize;
        let val_offset_buff = field.fill_buf().unwrap();
        debug_assert_eq!(val_offset_buff.len(), 4);
        let values =
            tiff_field::read_values(bytes, endianness, field_type, val_count, val_offset_buff)?;

        Ok(ifd::Entry::new(tag, values))
    });

    let mut field_errors = Vec::new();
    let entries: Vec<_> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(err) => {
                field_errors.push(err);
                None
            }
        })
        .collect();
    Ok((entries, field_errors))
}
