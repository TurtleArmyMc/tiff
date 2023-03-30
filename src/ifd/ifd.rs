use crate::types::{Byte, Long, Short, URational};

pub(crate) const ENTRY_COUNT_LEN: usize = 2;
pub(crate) const NEXT_IFD_OFFSET_LEN: usize = 4;

pub(crate) struct ImageFileDirectory {
    entries: Vec<IFDEntry>,
    next_directory_offset: Option<u32>,
}

impl ImageFileDirectory {
    fn new(entries: Vec<IFDEntry>, next_directory_offset: Option<u32>) -> Self {
        Self {
            entries,
            next_directory_offset,
        }
    }

    fn num_entries(&self) -> Short {
        self.entries.len().try_into().unwrap()
    }
}

pub(crate) struct IFDEntry {
    tag: IfdFieldTag,
    values: IfdFieldValues,
}

impl IFDEntry {
    pub(crate) fn new(tag: IfdFieldTag, values: IfdFieldValues) -> Self {
        Self { tag, values }
    }

    pub(crate) const LEN_BYTES: usize = 12;

    pub(crate) fn tag(&self) -> IfdFieldTag {
        self.tag
    }

    pub(crate) fn values(&self) -> &IfdFieldValues {
        &self.values
    }
}

/// Represents the id representing each ifd field type.
#[repr(u16)]
#[derive(Clone, Copy)]
pub(crate) enum IfdFieldType {
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

pub(crate) enum IfdFieldValues {
    Bytes(Vec<Byte>),
    // According to the spec, only storing a single string is preferred when possible.
    ASCII(String),
    Shorts(Vec<Short>),
    Longs(Vec<Long>),
    Rationals(Vec<URational>),
}

#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum IfdFieldTag {
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
    TileWidth = 322,
    TileLength = 323,
    TileOffsets = 324,
    TileByteCounts = 325,
    JPEGProc = 512,
    JPEGQTables = 519,
    JPEGDCTables = 520,
    JPEGACTables = 521,
}
