/// [`super::ifd::IfdFieldTag::PhotometricInterpretation`]
#[repr(u16)]
pub enum PhotometricInterpretation {
    WhiteIsZero = 0,
    BlackIsZero = 1,
}

/// [`super::ifd::IfdFieldTag::Compression`]
#[repr(u16)]
pub enum Compression {
    NoCompression = 1,
    /// CCITT Group 3 1-Dimensional Modified Huffman run length encoding
    Huffman = 2,
    PackBits = 32773,
}

/// [`super::ifd::IfdFieldTag::ResolutionUnit`]
#[repr(u16)]
pub enum ResolutionUnit {
    /// No absolute unit of measurement
    NoUnit = 1,
    Inch = 2,
    Centimeter = 3,
}
impl Default for ResolutionUnit {
    fn default() -> Self {
        Self::Inch
    }
}
