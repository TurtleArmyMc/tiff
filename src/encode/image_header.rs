use byteorder::{BigEndian, ByteOrder, LittleEndian, NativeEndian, WriteBytesExt};

pub trait EncodeEndianness: ByteOrder + private::EndiannessSentinel {}
impl EncodeEndianness for LittleEndian {}
impl EncodeEndianness for BigEndian {}

pub(crate) const HEADER_SIZE: usize = 8;

pub(crate) fn encode_header<E: EncodeEndianness>(wrt: &mut Vec<u8>) {
    // Endianness
    wrt.write_u16::<NativeEndian>(E::get_sentinel())
        .unwrap();
    // Magic number
    wrt.write_u16::<E>(42).unwrap();
    // Byte offset of first IFD (immediately after the header). This can be
    // overwritten later if necessary.
    wrt.write_u32::<E>(8).unwrap();
}

mod private {
    use byteorder::{BigEndian, ByteOrder, LittleEndian};

    // Sealed
    pub trait EndiannessSentinel: ByteOrder {
        fn get_sentinel() -> u16;
    }

    impl EndiannessSentinel for LittleEndian {
        fn get_sentinel() -> u16 {
            u16::from_le_bytes(['I' as u8, 'I' as u8])
        }
    }

    impl EndiannessSentinel for BigEndian {
        fn get_sentinel() -> u16 {
            u16::from_le_bytes(['M' as u8, 'M' as u8])
        }
    }
}
