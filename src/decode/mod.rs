mod grayscale_4bit;
mod grayscale_8bit;
mod palette_color;
mod rgb;

use std::{borrow::BorrowMut, ffi::CStr, io, slice};

use byteordered::{Endian, Endianness};

use crate::{colors, ifd, types::URational, Image};

#[allow(unused)]
pub fn decode_images(bytes: &[u8]) -> Result<DecodeResult, DecodeError> {
    // Read endianness from header and check magic number
    let endianness = match bytes {
        // II
        [73, 73, 42, 00] => Endianness::Little,
        // MM
        [77, 77, 00, 42] => Endianness::Big,
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
        ifd_index = ifd.next_ifd_index(endianness);

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

pub enum DecodeError {
    InvalidFiletype,
    UnknownFieldTag(u16),
    UnknownFieldType(u16),
    CantReadField,
    NoPhotometricInterpretation { ifd_index: usize },
    LoopingIfdIndices,
    TiffFieldError,
    InvalidImageFieldDirectory(usize),
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

fn decode_image(
    bytes: &[u8],
    endianness: Endianness,
    ifd: Ifd<'_>,
) -> Result<(DecodedImage, Vec<DecodeError>), DecodeError> {
    // TODO: Report field_errors
    let (fields, field_errors) = read_image_field_directory(bytes, endianness, ifd)?;

    let raw_photo_interp_values = fields.iter().find_map(|field| match field.tag() {
        ifd::Tag::PhotometricInterpretation => Some(field.values()),
        _ => None,
    });

    let raw_photo_interp = match raw_photo_interp_values {
        Some(ifd::Values::Shorts(values)) => match &values[..] {
            &[raw_photo_interp] => Ok(raw_photo_interp),
            // TODO: Make error more descriptive
            // _ => Err(DecodeError::InvalidTagValueCount {
            //     tag: ifd::Tag::PhotometricInterpretation,
            //     n: values.len(),
            // }),
            _ => Err(DecodeError::TiffFieldError),
        },
        // TODO: Make error more descriptive
        // Some(values) => Err(DecodeError::InvalidTypeForTag {
        //     tag: ifd::Tag::PhotometricInterpretation,
        //     value_type: values.field_type_tag(),
        // }),
        Some(_) => Err(DecodeError::TiffFieldError),
        None => Err(DecodeError::NoPhotometricInterpretation {
            ifd_index: ifd.ifd_index,
        }),
    }?;

    let photo_interp = ifd::tags::PhotometricInterpretation::from_repr(raw_photo_interp)
        // TODO: Make error more descriptive
        // .ok_or_else(|| DecodeError::InvalidTagValues {
        //     tag: ifd::Tag::PhotometricInterpretation,
        //     values: ifd::Values::Shorts(vec![raw_photo_interp]),
        // })?;
        .ok_or(DecodeError::TiffFieldError)?;

    let image = match photo_interp {
        ifd::tags::PhotometricInterpretation::WhiteIsZero
        | ifd::tags::PhotometricInterpretation::BlackIsZero => decode_bw_image()?,
        ifd::tags::PhotometricInterpretation::RGB => rgb::decode_image()?,
        ifd::tags::PhotometricInterpretation::PaletteColor => palette_color::decode_image()?,
    };

    Ok((image, field_errors))
}

#[derive(Clone, Copy)]
struct Ifd<'a> {
    ifd_bytes: &'a [u8],
    ifd_index: usize,
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
        Ok(Self {
            ifd_bytes,
            ifd_index,
        })
    }

    fn next_ifd_index(&self, endianness: Endianness) -> usize {
        endianness
            .read_u32(&self.ifd_bytes[self.ifd_bytes.len() - 4..])
            .unwrap() as usize
    }

    fn fields(&self) -> slice::ChunksExact<'_, u8> {
        self.ifd_bytes.chunks_exact(ifd::Entry::LEN)
    }
}

fn decode_bw_image() -> Result<DecodedImage, DecodeError> {
    todo!()
}

fn read_image_field_directory(
    bytes: &[u8],
    endianness: Endianness,
    ifd: Ifd<'_>,
) -> Result<(Vec<ifd::Entry>, Vec<DecodeError>), DecodeError> {
    let entries = ifd.fields().map(|field| -> Result<_, DecodeError> {
        let mut field = io::Cursor::new(field);
        let tag = endianness.read_u16(field.borrow_mut()).unwrap();
        let field_type = endianness.read_u16(field.borrow_mut()).unwrap();
        let val_count = endianness.read_u32(field.borrow_mut()).unwrap();
        let val_offset = endianness.read_u32(field.borrow_mut()).unwrap();

        let tag = ifd::Tag::from_repr(tag).ok_or(DecodeError::UnknownFieldTag(tag))?;
        let field_type =
            ifd::Type::from_repr(field_type).ok_or(DecodeError::UnknownFieldType(field_type))?;

        let values = read_field_values(
            bytes,
            endianness,
            field_type,
            val_count,
            val_offset as usize,
        )?;
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

fn read_field_values(
    bytes: &[u8],
    endianness: Endianness,
    valtype: ifd::Type,
    count: u32,
    index: usize,
) -> Result<ifd::Values, DecodeError> {
    match valtype {
        ifd::Type::Byte => bytes
            .get(index..index + count as usize)
            .map(Vec::from)
            .map(ifd::Values::Bytes)
            .ok_or(DecodeError::CantReadField),
        ifd::Type::Short => bytes
            .get(index..index + (count as usize * 2))
            .map(|bytes| bytes.chunks_exact(2))
            .map(|chunks| chunks.map(|chunk| endianness.read_u16(chunk).unwrap()))
            .map(Vec::from_iter)
            .map(ifd::Values::Shorts)
            .ok_or(DecodeError::CantReadField),
        ifd::Type::Long => bytes
            .get(index..index + (count as usize * 4))
            .map(|bytes| bytes.chunks_exact(4))
            .map(|chunks| chunks.map(|chunk| endianness.read_u32(chunk).unwrap()))
            .map(Vec::from_iter)
            .map(ifd::Values::Longs)
            .ok_or(DecodeError::CantReadField),
        ifd::Type::Rational => {
            let mut longs = bytes
                .get(index..index + (count as usize * 8))
                .map(|bytes| bytes.chunks_exact(4))
                .map(|chunks| chunks.map(|chunk| endianness.read_u32(chunk).unwrap()))
                .ok_or(DecodeError::CantReadField)?;
            let mut rats = Vec::new();
            rats.reserve_exact(count as usize);
            while let (Some(numerator), Some(denominator)) = (longs.next(), longs.next()) {
                rats.push(URational {
                    numerator,
                    denominator,
                })
            }
            Ok(ifd::Values::Rationals(rats))
        }
        ifd::Type::ASCII => {
            if count <= 1 {
                bytes
                    .get(index..)
                    .and_then(|str_bytes| CStr::from_bytes_until_nul(str_bytes).ok())
                    .and_then(|cstr| cstr.to_str().ok()) // TODO: Decode to exact CStrings
                    .map(|str| str.to_owned())
                    .map(ifd::Values::ASCII)
                    .ok_or(DecodeError::CantReadField)
            } else {
                todo!("support decoding multiple strings")
            }
        }
    }
}
