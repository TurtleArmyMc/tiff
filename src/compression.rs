use crate::{colors, decode, encode, ifd};

pub trait Compression<C: colors::Color>:
    encode::compression::sealed::CompressionImpl + decode::compression::sealed::DecompressionImpl
{
    fn compression_type_tag(&self) -> ifd::tags::Compression;
}

#[derive(Clone, Copy)]
pub struct NoCompression;

#[derive(Clone, Copy)]
pub struct PackBits;

#[derive(Clone, Copy)]
pub struct Lzw;

impl<C: colors::Color> Compression<C> for NoCompression {
    fn compression_type_tag(&self) -> ifd::tags::Compression {
        ifd::tags::Compression::NoCompression
    }
}

impl<C: colors::Color> Compression<C> for PackBits {
    fn compression_type_tag(&self) -> ifd::tags::Compression {
        ifd::tags::Compression::PackBits
    }
}

impl<C: colors::Color> Compression<C> for Lzw {
    fn compression_type_tag(&self) -> ifd::tags::Compression {
        ifd::tags::Compression::Lzw
    }
}
