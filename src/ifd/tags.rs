/// [`super::ifd::IfdFieldTag::PhotometricInterpretation`]
#[derive(strum::FromRepr, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PhotometricInterpretation {
    WhiteIsZero = 0,
    BlackIsZero = 1,
    RGB = 2,
    PaletteColor = 3,
}

/// [`super::ifd::IfdFieldTag::Compression`]
#[derive(strum::FromRepr, Clone, Copy)]
#[repr(u16)]
pub enum Compression {
    NoCompression = 1,
    /// CCITT Group 3 1-Dimensional Modified Huffman run length encoding
    Huffman = 2,
    Lzw = 5,
    PackBits = 32773,
}

impl Default for Compression {
    fn default() -> Self {
        Self::NoCompression
    }
}

/// [`super::ifd::IfdFieldTag::ResolutionUnit`]
#[derive(strum::FromRepr, Clone, Copy)]
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
