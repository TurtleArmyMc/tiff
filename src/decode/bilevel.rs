use super::{DecodeError, ImageInfo};
use crate::{colors, ifd, Image};

pub(crate) fn decode_image(
    fields: Vec<ifd::Entry>,
    info: ImageInfo,
    white_is_zero: bool,
) -> Result<Image<colors::Bilevel>, DecodeError> {
    todo!()
}
