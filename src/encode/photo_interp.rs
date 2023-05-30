use crate::colors;

pub trait PhotometricInterpretation<C: colors::Color>:
    private::PhotometricInterpretationImpl<C>
{
}

#[derive(Clone, Copy)]
pub struct BlackIsZero;
#[derive(Clone, Copy)]
pub struct WhiteIsZero;

impl PhotometricInterpretation<colors::Bilevel> for BlackIsZero {}
impl PhotometricInterpretation<colors::Bilevel> for WhiteIsZero {}
impl PhotometricInterpretation<colors::Grayscale4Bit> for BlackIsZero {}
impl PhotometricInterpretation<colors::Grayscale4Bit> for WhiteIsZero {}
impl PhotometricInterpretation<colors::Grayscale8Bit> for BlackIsZero {}
impl PhotometricInterpretation<colors::Grayscale8Bit> for WhiteIsZero {}

pub(crate) mod private {
    use crate::{colors, ifd};

    use super::{BlackIsZero, WhiteIsZero};

    pub trait PhotometricInterpretationTag {
        fn tag(&self) -> ifd::tags::PhotometricInterpretation;
    }

    pub trait PhotometricInterpretationImpl<C: colors::Color>:
        PhotometricInterpretationTag
    {
        fn encode_pixel(&self, pixel: C) -> C::EncodeTo;
    }

    impl PhotometricInterpretationTag for BlackIsZero {
        fn tag(&self) -> ifd::tags::PhotometricInterpretation {
            ifd::tags::PhotometricInterpretation::BlackIsZero
        }
    }

    impl PhotometricInterpretationTag for WhiteIsZero {
        fn tag(&self) -> ifd::tags::PhotometricInterpretation {
            ifd::tags::PhotometricInterpretation::WhiteIsZero
        }
    }
}
