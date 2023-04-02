pub trait Compression: private::Sealed {}

#[derive(Clone, Copy)]
pub struct NoCompression;
impl Compression for NoCompression {}

pub(crate) mod private {
    use crate::ifd;

    use super::NoCompression;

    pub trait Sealed: crate::encode::bilevel::private::BilevelEncoder {
        fn tag(&self) -> ifd::tags::Compression;
    }

    impl Sealed for NoCompression {
        fn tag(&self) -> ifd::tags::Compression {
            ifd::tags::Compression::NoCompression
        }
    }
}
