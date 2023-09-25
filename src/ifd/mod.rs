pub(crate) mod tags;

use crate::types::{Byte, Long, Short, URational};

/// Length of the entry count field in the IFD in bytes
pub(crate) const ENTRY_COUNT_LEN: usize = 2;
/// Length of the offset to the next IFD at the end of each IFD in bytes
pub(crate) const NEXT_IFD_OFFSET_LEN: usize = 4;

/// Returns the length of the IFD
pub(crate) const fn get_len(fields: usize) -> usize {
    ENTRY_COUNT_LEN + fields * Entry::LEN + NEXT_IFD_OFFSET_LEN
}

pub(crate) struct Entry {
    tag: Tag,
    values: Values,
}

impl Entry {
    /// Length of each whole in bytes
    pub(crate) const LEN: usize = 12;

    pub(crate) fn new(tag: Tag, values: Values) -> Self {
        Self { tag, values }
    }

    pub(crate) fn tag(&self) -> Tag {
        self.tag
    }

    pub(crate) fn values(&self) -> &Values {
        &self.values
    }
}

/// Represents the id representing each ifd field type.
#[repr(u16)]
#[derive(strum::FromRepr, Clone, Copy)]
pub(crate) enum Type {
    /// 8-bit unsigned integer.
    Byte = 1,
    /// 8-bit byte that contains a 7-bit ASCII code; the last byte must be NUL (binary zero).
    ASCII = 2,
    /// 16-bit (2-byte) unsigned integer.
    Short = 3,
    /// 32-bit (4-byte) unsigned integer.
    Long = 4,
    /// Two LONGs: the first represents the numerator of a fraction; the second, the denominator.
    Rational = 5,
    // End of pre-TIFF 6 types

    // SByte,
    // Undefined,
    // SShort,
    // SLong,
    // SRational,
    // Float,
    // Double,
}

pub(crate) enum Values {
    Bytes(Vec<Byte>),
    // According to the spec, only storing a single string is preferred when possible.
    ASCII(String),
    Shorts(Vec<Short>),
    Longs(Vec<Long>),
    Rationals(Vec<URational>),
}

impl Values {
    pub(crate) const fn field_type_tag(&self) -> Type {
        match self {
            Values::Bytes(_) => Type::Byte,
            Values::ASCII(_) => Type::ASCII,
            Values::Shorts(_) => Type::Short,
            Values::Longs(_) => Type::Long,
            Values::Rationals(_) => Type::Rational,
        }
    }

    pub(crate) fn num_values(&self) -> Long {
        match self {
            Values::Bytes(bytes) => bytes.len().try_into().unwrap(),
            Values::ASCII(_) => 1,
            Values::Shorts(short) => short.len().try_into().unwrap(),
            Values::Longs(long) => long.len().try_into().unwrap(),
            Values::Rationals(rational) => rational.len().try_into().unwrap(),
        }
    }
}

#[repr(u16)]
#[derive(strum::FromRepr, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Tag {
    NewSubFileType = 254,
    ImageWidth = 256,
    ImageLength = 257,
    BitsPerSample = 258,
    Compression = 259,
    PhotometricInterpretation = 262,
    /// For each strip, the byte offset of that strip.
    StripOffsets = 273,
    SamplesPerPixel = 277,
    /// The number of rows in each strip (except possibly the last strip.)
    RowsPerStrip = 278,
    /// For each strip, the number of bytes in that strip after any compression.
    StripByteCounts = 279,
    /// The number of pixels per ResolutionUnit in the ImageWidth (typically, horizontal) direction.
    XResolution = 282,
    /// The number of pixels per ResolutionUnit in the ImageLength (typically, vertical) direction.
    YResolution = 283,
    PlanarConfiguration = 284,
    ResolutionUnit = 296,
    ColorMap = 320,
    TileWidth = 322,
    TileLength = 323,
    TileOffsets = 324,
    TileByteCounts = 325,
    JPEGProc = 512,
    JPEGQTables = 519,
    JPEGDCTables = 520,
    JPEGACTables = 521,
}
