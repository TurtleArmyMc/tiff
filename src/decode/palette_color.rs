use super::{DecodeError, ImageInfo};
use crate::{colors, ifd, Image};

pub(crate) fn decode_image(
    fields: Vec<ifd::Entry>,
    info: ImageInfo,
) -> Result<Image<colors::RGB>, DecodeError> {
    todo!()
}
