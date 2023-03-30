use byteorder::{BigEndian, ByteOrder, LittleEndian};

pub trait EncodeEndianness: ByteOrder + private::EndiannessSentinel {}
impl EncodeEndianness for LittleEndian {}
impl EncodeEndianness for BigEndian {}

pub(crate) const LEN: usize = 8;

mod private {
    use byteorder::{BigEndian, ByteOrder, LittleEndian};

    // Sealed
    pub trait EndiannessSentinel: ByteOrder {
        fn get_sentinel() -> u8;
    }

    impl EndiannessSentinel for LittleEndian {
        fn get_sentinel() -> u8 {
            'I' as u8
        }
    }

    impl EndiannessSentinel for BigEndian {
        fn get_sentinel() -> u8 {
            'M' as u8
        }
    }
}
