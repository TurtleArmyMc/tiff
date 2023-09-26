use super::{DecodeError, ImageInfo};
use crate::{
    colors::{self, Grayscale8Bit},
    ifd, Image,
};

pub(crate) fn decode_image(
    bytes: &[u8],
    fields: Vec<ifd::Entry>,
    info: ImageInfo,
    white_is_zero: bool,
) -> Result<Image<colors::Grayscale8Bit>, DecodeError> {
    let mut pixels = Vec::new();
    pixels.reserve_exact(info.width * info.height);

    if (info.height != info.rows_per_strip * info.strip_offsets.len())
        || (info.strip_offsets.len() != info.strip_byte_counts.len())
    {
        return Err(DecodeError::CantReadImage);
    }

    let to_pixel = if white_is_zero {
        |byte| Grayscale8Bit(u8::MAX - byte)
    } else {
        |byte| Grayscale8Bit(byte)
    };

    for (offset, strip_byte_count) in info
        .strip_offsets
        .into_iter()
        .zip(info.strip_byte_counts.into_iter())
    {
        let strip = bytes
            .get(offset..offset + strip_byte_count)
            .ok_or(DecodeError::CantReadImage)?;
        pixels.extend(info.decompressor.decompress(strip).map(to_pixel));
    }

    Image::try_new(pixels, info.width, info.height).map_err(|_| DecodeError::CantReadImage)
}
