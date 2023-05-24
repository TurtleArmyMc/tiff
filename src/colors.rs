use std::{
    collections::{hash_map, HashMap},
    iter::repeat,
};

use crate::{types::Short, Image};

pub trait Color: Copy {}

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
impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone)]
pub struct ColorMap {
    to_rgb: Vec<RGB>,
    to_inx: HashMap<RGB, u8>,
}
impl ColorMap {
    pub fn new() -> Self {
        Self {
            to_rgb: Vec::new(),
            to_inx: HashMap::new(),
        }
    }

    /// Returns the palettized pixels.
    ///
    /// # Panics
    ///
    /// Panics if there are too many colors to palettize (more than 256).
    pub fn palettize_pixels<'a>(&'a mut self, pixels: &[RGB]) -> Vec<PaletteColor<'a>> {
        self.try_palettize_pixels(pixels)
            .expect("too many colors for palette in pixels")
    }

    /// Returns the palettized pixels, or None if there are too many colors to palettize (more than 256).
    /// If palettization fails, the map will be filled with 256 colors.
    pub fn try_palettize_pixels<'a>(&'a mut self, pixels: &[RGB]) -> Option<Vec<PaletteColor<'a>>> {
        for pixel in pixels {
            if self.get_or_create_inx(*pixel) == None {
                return None;
            }
        }
        Some(
            pixels
                .iter()
                .copied()
                .map(|pixel| PaletteColor::new(self, self.get_inx(pixel).unwrap()))
                .collect(),
        )
    }

    pub(crate) fn can_be_4bit(&self) -> bool {
        self.to_rgb.len() <= 16
    }

    pub(crate) fn create_colormap_vec(&self) -> Vec<Short> {
        let colors = self
            .to_rgb
            .iter()
            .copied()
            .chain(repeat(RGB::new(0, 0, 0)).take(256 - self.to_rgb.len()));
        colors
            .clone()
            .map(|color| color.r as Short)
            .chain(colors.clone().map(|color| color.g as Short))
            .chain(colors.clone().map(|color| color.b as Short))
            .collect()
    }

    fn get_inx(&self, c: RGB) -> Option<u8> {
        self.to_inx.get(&c).map(|inx| *inx as u8)
    }

    fn get_or_create_inx(&mut self, c: RGB) -> Option<u8> {
        match self.to_inx.entry(c) {
            hash_map::Entry::Occupied(e) => Some(*e.get()),
            hash_map::Entry::Vacant(e) => {
                if self.to_rgb.len() >= 256 {
                    None
                } else {
                    self.to_rgb.push(c);
                    Some(*e.insert((self.to_rgb.len() - 1) as u8))
                }
            }
        }
    }
}

impl<'a> Image<PaletteColor<'a>> {
    pub fn can_be_4bit(&self) -> bool {
        self.pixels.first().unwrap().can_be_4bit()
    }

    pub(crate) fn get_colormap(&self) -> &ColorMap {
        self.pixels.first().unwrap().map
    }
}

#[derive(Clone, Copy)]
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

    pub(crate) fn can_be_4bit(&self) -> bool {
        self.map.can_be_4bit()
    }
}
