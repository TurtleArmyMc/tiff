pub(crate) mod sealed {
    use crate::compression::{Lzw, NoCompression, PackBits};

    pub trait DecompressionImpl {}

    impl DecompressionImpl for NoCompression {}
    impl DecompressionImpl for PackBits {}
    impl DecompressionImpl for Lzw {}
}
