use std::slice::{ChunksExact, ChunksExactMut};

use colors::Color;

pub mod colors;
pub mod encode;
pub mod ifd;
mod types;

pub struct Image<C: Color> {
    /// Pixels arranged left to right, then top to bottom
    pixels: Vec<C>,
    width: usize,
    height: usize,
}

impl<C: Color> Image<C> {
    /// # Panics
    ///
    /// Panics if the number of elements in pixels is 0 or not equal to width * height
    pub fn new(pixels: Vec<C>, width: usize, height: usize) -> Self {
        if width * height != pixels.len() {
            panic!(
                "expected {width}*{height} ({}) pixels but got {}",
                width * height,
                pixels.len()
            )
        } else if pixels.len() == 0 {
            panic!("image can not be 0x0 pixels")
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    pub fn pixels(&self) -> ChunksExact<C> {
        self.pixels.chunks_exact(self.width)
    }

    pub fn pixels_mut(&mut self) -> ChunksExactMut<C> {
        self.pixels.chunks_exact_mut(self.width)
    }

    pub fn pixels_vec(self) -> Vec<C> {
        self.pixels
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn pixel_count(&self) -> usize {
        self.width * self.height
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
