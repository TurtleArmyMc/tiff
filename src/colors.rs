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
