use std::ffi::CStr;

use byteordered::{Endian, Endianness};

use crate::{
    ifd,
    types::{Long, Short, URational},
};

use super::DecodeError;

pub(crate) fn read_values(
    bytes: &[u8],
    endianness: Endianness,
    valtype: ifd::Type,
    count: usize,
    offset_buff: &[u8],
) -> Result<ifd::Values, DecodeError> {
    match valtype {
        ifd::Type::Byte => {
            let val_buff = if count <= 4 {
                &offset_buff[..count]
            } else {
                let index = endianness.read_u32(offset_buff).unwrap() as usize;
                bytes
                    .get(index..index + count)
                    .ok_or(DecodeError::CantReadField)?
            };
            Ok(ifd::Values::Bytes(Vec::from(val_buff)))
        }
        ifd::Type::Short => {
            let val_buff = if count <= 2 {
                &offset_buff[..count * 2]
            } else {
                let index = endianness.read_u32(offset_buff).unwrap() as usize;
                bytes
                    .get(index..index + count * 2)
                    .ok_or(DecodeError::CantReadField)?
            };
            Ok(ifd::Values::Shorts(
                val_buff
                    .chunks_exact(2)
                    .into_iter()
                    .map(|chunk| endianness.read_u16(chunk).unwrap())
                    .collect(),
            ))
        }
        ifd::Type::Long => {
            let val_buff = if count <= 1 {
                offset_buff
            } else {
                let index = endianness.read_u32(offset_buff).unwrap() as usize;
                bytes
                    .get(index..index + count * 4)
                    .ok_or(DecodeError::CantReadField)?
            };
            Ok(ifd::Values::Longs(
                val_buff
                    .chunks_exact(4)
                    .into_iter()
                    .map(|chunk| endianness.read_u32(chunk).unwrap())
                    .collect(),
            ))
        }
        ifd::Type::Rational => {
            let index = endianness.read_u32(offset_buff).unwrap() as usize;
            let mut longs = bytes
                .get(index..index + count * 8)
                .map(|bytes| bytes.chunks_exact(4))
                .map(|chunks| chunks.map(|chunk| endianness.read_u32(chunk).unwrap()))
                .ok_or(DecodeError::CantReadField)?;
            let mut rats = Vec::new();
            rats.reserve_exact(count);
            while let (Some(numerator), Some(denominator)) = (longs.next(), longs.next()) {
                rats.push(URational {
                    numerator,
                    denominator,
                })
            }
            Ok(ifd::Values::Rationals(rats))
        }
        ifd::Type::ASCII => {
            let index = endianness.read_u32(offset_buff).unwrap() as usize;
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

pub(crate) fn find(fields: &[ifd::Entry], tag: ifd::Tag) -> Option<&ifd::Values> {
    fields
        .iter()
        .find(|field| field.tag() == tag)
        .map(|field| field.values())
}

pub(crate) fn find_required(
    fields: &[ifd::Entry],
    tag: ifd::Tag,
) -> Result<&ifd::Values, DecodeError> {
    find(fields, tag).ok_or(DecodeError::MissingRequiredField)
    // .ok_or(DecodeError::MissingRequiredField { tag, ifd_index })
}

pub(crate) fn take(fields: &mut Vec<ifd::Entry>, tag: ifd::Tag) -> Option<ifd::Values> {
    fields
        .iter()
        .position(|field| field.tag() == tag)
        .map(|pos| fields.swap_remove(pos).into())
}

pub(crate) fn take_required(
    fields: &mut Vec<ifd::Entry>,
    tag: ifd::Tag,
) -> Result<ifd::Values, DecodeError> {
    take(fields, tag).ok_or(DecodeError::MissingRequiredField)
    // .ok_or(DecodeError::MissingRequiredField { tag, ifd_index })
}

// TODO: Use DecodeError::InvalidTagValueCount and DecodeError::InvalidTypeForTag
pub(crate) fn read_single_short(values: &ifd::Values) -> Result<Short, DecodeError> {
    match values {
        ifd::Values::Shorts(shorts) => match &shorts[..] {
            &[short] => Ok(short),
            _ => Err(DecodeError::TiffFieldError),
        },
        _ => Err(DecodeError::TiffFieldError),
    }
}

// TODO: Use DecodeError::InvalidTagValueCount and DecodeError::InvalidTypeForTag
/// Reads a single short or long
pub(crate) fn read_single(values: &ifd::Values) -> Result<Long, DecodeError> {
    match values {
        ifd::Values::Shorts(shorts) => match &shorts[..] {
            &[short] => Ok(short as Long),
            _ => Err(DecodeError::TiffFieldError),
        },
        ifd::Values::Longs(longs) => match &longs[..] {
            &[long] => Ok(long),
            _ => Err(DecodeError::TiffFieldError),
        },
        _ => Err(DecodeError::TiffFieldError),
    }
}

pub(crate) fn read_single_rational(values: &ifd::Values) -> Result<URational, DecodeError> {
    match values {
        ifd::Values::Rationals(rationals) => match &rationals[..] {
            &[rational] => Ok(rational),
            _ => Err(DecodeError::TiffFieldError),
        },
        _ => Err(DecodeError::TiffFieldError),
    }
}

/// Reads shorts or longs
pub(crate) fn as_usizes(values: ifd::Values) -> Result<Vec<usize>, DecodeError> {
    match values {
        ifd::Values::Shorts(shorts) => Ok(shorts.into_iter().map(usize::from).collect()),
        ifd::Values::Longs(longs) => Ok(longs.into_iter().map(|long| long as usize).collect()),
        _ => Err(DecodeError::TiffFieldError),
    }
}
