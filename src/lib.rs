use std::{error::Error, fmt::Display, slice::ChunksExact};

use colors::Color;

pub mod colors;
mod compression;
pub mod decode;
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
    /// Pixels are arranged left to right, then top to bottom.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in pixels is 0 or not equal to width * height
    pub fn new(pixels: Vec<C>, width: usize, height: usize) -> Self {
        Self::try_new(pixels, width, height).unwrap()
    }

    /// Pixels are arranged left to right, then top to bottom.
    pub fn try_new(pixels: Vec<C>, width: usize, height: usize) -> Result<Self, ImageCreateError> {
        if width * height != pixels.len() {
            Err(ImageCreateError::DimensionMismatch {
                width,
                height,
                pixel_count: pixels.len(),
            })
        } else if pixels.len() == 0 {
            Err(ImageCreateError::NoPixels)
        } else {
            Ok(Self {
                pixels,
                width,
                height,
            })
        }
    }

    pub fn iter_pixels(&self) -> impl Iterator<Item = impl Iterator<Item = C::ViewAs> + '_> + '_ {
        self.pixels
            .chunks_exact(self.width)
            .map(|row| row.iter().map(C::view))
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn rows(&self) -> ChunksExact<C> {
        self.pixels.chunks_exact(self.width)
    }
}

impl<C: Color<ViewAs = C>> Image<C> {
    pub fn into_pixels(Self { pixels, .. }: Self) -> Vec<C::ViewAs> {
        pixels
    }
}

#[derive(Debug)]
pub enum ImageCreateError {
    DimensionMismatch {
        width: usize,
        height: usize,
        pixel_count: usize,
    },
    NoPixels,
}

impl Display for ImageCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageCreateError::DimensionMismatch {
                width,
                height,
                pixel_count,
            } => write!(
                f,
                "expected {width}*{height} ({}) pixels but got {pixel_count}",
                width * height,
            ),
            ImageCreateError::NoPixels => write!(f, "image can not be 0x0 pixels"),
        }
    }
}

impl Error for ImageCreateError {}
