pub trait Color: Copy {}

#[derive(Clone, Copy)]
pub enum Bilevel {
    Black,
    White,
}
impl Color for Bilevel {}
