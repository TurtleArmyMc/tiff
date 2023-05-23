pub trait Color: Copy {}

#[derive(Clone, Copy)]
pub enum Bilevel {
    Black,
    White,
}
impl Color for Bilevel {}

#[derive(Clone, Copy)]
pub struct Grayscale8Bit(pub u8);
impl Color for Grayscale8Bit {}

#[derive(Clone, Copy)]
pub struct Grayscale4Bit(u8);
impl Color for Grayscale4Bit {}
impl Grayscale4Bit {
    pub fn new(pixel: u8) -> Self {
        if pixel > 0b1111 {
            panic!("4bit grayscale pixel can not be more than {}", 0b1111)
        }
        Self(pixel)
    }

    pub fn new_checked(pixel: u8) -> Option<Self> {
        if pixel > 0b1111 {
            None
        } else {
            Some(Self(pixel))
        }
    }

    pub fn value(self) -> u8 {
        self.0
    }
}
