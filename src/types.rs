pub(crate) type Byte = u8;
pub(crate) type Short = u16;
pub(crate) type Long = u32;

#[derive(Clone, Copy)]
pub(crate) struct URational {
    pub numerator: u32,
    pub denominator: u32,
}

impl URational {
    pub(crate) fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}
