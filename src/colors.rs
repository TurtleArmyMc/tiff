use std::iter::repeat;

use crate::{types::Short, Image};

pub trait Color {}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum Bilevel {
    Black,
    White,
}
impl Color for Bilevel {}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Grayscale8Bit(pub u8);
impl Color for Grayscale8Bit {}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
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

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Color for RGB {}
impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone)]
pub struct ColorMap {
    to_rgb: Vec<RGB>,
}
impl ColorMap {
    pub const MAX_COLORS: usize = 256;

    pub fn new() -> Self {
        Self { to_rgb: Vec::new() }
    }

    /// Returns a palettized image with the given pixels, using only colors in
    /// the palette.
    ///
    /// Returns `None` if a pixel is not in the palette.
    pub fn try_new_exact_image<'a>(
        &'a self,
        pixels: &[RGB],
        width: usize,
        height: usize,
    ) -> Option<Image<PaletteColor<'a>>> {
        self.try_match_exact_pixels(pixels)
            .map(|palettized_pixels| Image::new(palettized_pixels, width, height))
    }

    /// Adds a color to the palette if it is not already in the palette, and
    /// there is space. Returns how many colors have been added if successfully
    /// added or if the color was already in the palette.
    pub fn try_add_color(&mut self, c: RGB) -> Option<usize> {
        self.get_or_create_inx(c).map(|_| self.to_rgb.len())
    }

    pub fn contains_color(&self, c: RGB) -> bool {
        self.to_rgb.contains(&c)
    }

    pub(crate) fn bits_per_palette_sample(&self) -> Short {
        if self.to_rgb.len() <= 16 {
            4
        } else {
            8
        }
    }

    pub(crate) fn create_colormap_vec(&self) -> Vec<Short> {
        let remaining = 2usize.pow(self.bits_per_palette_sample() as u32) - self.to_rgb.len();
        let colors = self
            .to_rgb
            .iter()
            .copied()
            .chain(repeat(RGB::new(0, 0, 0)).take(remaining));
        colors
            .clone()
            .map(|color| color.r as Short)
            .chain(colors.clone().map(|color| color.g as Short))
            .chain(colors.clone().map(|color| color.b as Short))
            .collect()
    }

    fn try_match_exact_pixels<'a>(&'a self, pixels: &[RGB]) -> Option<Vec<PaletteColor<'a>>> {
        let mut colors = Vec::new();
        colors.reserve_exact(pixels.len());
        for pixel in pixels {
            match self.get_inx(*pixel) {
                Some(color) => colors.push(PaletteColor::new(self, color)),
                None => return None,
            }
        }
        Some(colors)
    }

    fn get_inx(&self, c: RGB) -> Option<u8> {
        self.to_rgb
            .iter()
            .enumerate()
            .find(|(_, &pixel)| c == pixel)
            .map(|(inx, _)| inx as u8)
    }

    fn get_or_create_inx(&mut self, c: RGB) -> Option<u8> {
        match self.get_inx(c) {
            None => {
                if self.to_rgb.len() >= Self::MAX_COLORS {
                    None
                } else {
                    self.to_rgb.push(c);
                    Some((self.to_rgb.len() - 1) as u8)
                }
            }
            inx => inx,
        }
    }
}

impl<'a> Image<PaletteColor<'a>> {
    pub fn bits_per_palette_sample(&self) -> Short {
        self.pixels.first().unwrap().bits_per_palette_sample()
    }

    pub(crate) fn get_colormap(&self) -> &ColorMap {
        self.pixels.first().unwrap().map
    }
}

/// A color referencing a [`ColorMap`].
/// An [`Image<PaletteColor>`] must be created using a [`ColorMap`].
pub struct PaletteColor<'a> {
    map: &'a ColorMap,
    inx: u8,
}
impl<'a> Color for PaletteColor<'a> {}
impl<'a> PaletteColor<'a> {
    fn new(map: &'a ColorMap, inx: u8) -> Self {
        Self { map, inx }
    }

    pub(crate) fn get_inx(&self) -> u8 {
        self.inx
    }

    pub(crate) fn bits_per_palette_sample(&self) -> Short {
        self.map.bits_per_palette_sample()
    }
}
