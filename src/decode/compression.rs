use crate::{
    compression::{Lzw, NoCompression, PackBits},
    ifd,
};

use self::sealed::DecompressionImpl;

use super::DecodeError;

pub(crate) fn tag_to_decompressor(
    tag: ifd::tags::Compression,
) -> Result<Box<dyn DecompressionImpl>, DecodeError> {
    match tag {
        ifd::tags::Compression::NoCompression => Ok(Box::new(NoCompression)),
        ifd::tags::Compression::Huffman => Err(DecodeError::UnsupportedCompressionType),
        ifd::tags::Compression::Lzw => Ok(Box::new(Lzw)),
        ifd::tags::Compression::PackBits => Ok(Box::new(PackBits)),
    }
}

pub(crate) mod sealed {
    use crate::compression::{Lzw, NoCompression, PackBits};

    pub trait DecompressionImpl {
        fn decompress<'a>(&self, bytes: &'a [u8]) -> Box<dyn Iterator<Item = u8> + 'a>;
    }

    impl DecompressionImpl for NoCompression {
        fn decompress<'a>(&self, bytes: &'a [u8]) -> Box<dyn Iterator<Item = u8> + 'a> {
            Box::new(bytes.into_iter().copied())
        }
    }

    impl DecompressionImpl for PackBits {
        fn decompress<'a>(&self, bytes: &'a [u8]) -> Box<dyn Iterator<Item = u8> + 'a> {
            todo!()
        }
    }

    impl DecompressionImpl for Lzw {
        fn decompress<'a>(&self, bytes: &'a [u8]) -> Box<dyn Iterator<Item = u8> + 'a> {
            todo!()
        }
    }
}
