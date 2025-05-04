use log::warn;
use once_cell::sync::Lazy;
use peniko::color::{AlphaColor, Srgb};
use qcms::DataType::CMYK;
use qcms::Transform;
use smallvec::SmallVec;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub(crate) type ColorComponents = SmallVec<[f32; 4]>;

#[derive(Clone, Debug)]
pub(crate) enum ColorSpace {
    DeviceCmyk,
    DeviceGray,
    DeviceRgb,
    ICCColor(ICCProfile),
}

#[derive(Clone, Debug)]
pub enum ColorType {
    DeviceRgb([f32; 3]),
    DeviceGray(f32),
    DeviceCmyk([f32; 4]),
    Icc(ICCProfile, ColorComponents),
}

struct ICCColorRepr {
    transform: Transform,
    number_components: usize,
}

#[derive(Clone)]
pub struct ICCProfile(Arc<ICCColorRepr>);

impl Debug for ICCProfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ICCColor {{..}}")
    }
}

impl ICCProfile {
    pub fn new(profile: &[u8], number_components: usize) -> Option<Self> {
        let input = qcms::Profile::new_from_slice(profile, false)?;
        let mut output = qcms::Profile::new_sRGB();
        output.precache_output_transform();

        let data_type = match number_components {
            1 => qcms::DataType::Gray8,
            3 => qcms::DataType::RGB8,
            4 => qcms::DataType::CMYK,
            _ => {
                warn!(
                    "unsupported number of components {} for ICC profile",
                    number_components
                );

                return None;
            }
        };

        let transform = Transform::new_to(
            &input,
            &output,
            data_type,
            qcms::DataType::RGB8,
            qcms::Intent::default(),
        )?;

        Some(Self(Arc::new(ICCColorRepr {
            transform,
            number_components,
        })))
    }

    pub(crate) fn to_srgb(&self, c: &[f32]) -> [u8; 3] {
        let mut srgb = [0, 0, 0];

        match self.0.number_components {
            1 => self.0.transform.convert(&[u8_to_f32(c[0])], &mut srgb),
            3 => self.0.transform.convert(
                &[u8_to_f32(c[0]), u8_to_f32(c[1]), u8_to_f32(c[2])],
                &mut srgb,
            ),
            4 => self.0.transform.convert(
                &[
                    u8_to_f32(c[0]),
                    u8_to_f32(c[1]),
                    u8_to_f32(c[2]),
                    u8_to_f32(c[3]),
                ],
                &mut srgb,
            ),
            _ => unreachable!(),
        }

        srgb
    }
}

fn u8_to_f32(val: f32) -> u8 {
    (val * 255.0 + 0.5) as u8
}

#[derive(Clone, Debug)]
pub struct Color {
    color_type: ColorType,
    opacity: f32,
}

impl Color {
    pub(crate) fn from_pdf(color_space: ColorSpace, c: &ColorComponents, opacity: f32) -> Self {
        let c_type = match color_space {
            ColorSpace::DeviceCmyk => ColorType::DeviceCmyk([c[0], c[1], c[2], c[3]]),
            ColorSpace::DeviceGray => ColorType::DeviceGray(c[0]),
            ColorSpace::DeviceRgb => ColorType::DeviceRgb([c[0], c[1], c[2]]),
            ColorSpace::ICCColor(icc) => ColorType::Icc(icc, c.clone()),
        };

        Self {
            color_type: c_type,
            opacity,
        }
    }

    pub fn to_rgba(&self) -> AlphaColor<Srgb> {
        // Conversions according to section 10.4 in the spec.
        match &self.color_type {
            ColorType::DeviceRgb(r) => AlphaColor::new([r[0], r[1], r[2], self.opacity]),
            ColorType::DeviceGray(g) => AlphaColor::new([*g, *g, *g, self.opacity]),
            ColorType::DeviceCmyk(c) => {
                let opacity = u8_to_f32(self.opacity);
                let mut srgb = CMYK_TRANSFORM.to_srgb(&c[..]);

                let res = AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity);

                res
            }
            ColorType::Icc(icc, c) => {
                let opacity = u8_to_f32(self.opacity);
                let mut srgb = icc.to_srgb(&c[..]);

                let res = AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity);

                res
            }
        }
    }
}

static CMYK_TRANSFORM: Lazy<ICCProfile> = Lazy::new(|| {
    ICCProfile::new(
        include_bytes!("../../assets/CGATS001Compat-v2-micro.icc"),
        4,
    )
    .unwrap()
});
