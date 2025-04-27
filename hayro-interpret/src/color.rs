use peniko::color::{AlphaColor, Srgb};
use smallvec::SmallVec;

pub(crate) type ColorComponents = SmallVec<[f32; 4]>;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ColorSpace {
    DeviceCmyk,
    DeviceGray,
    DeviceRgb
}

#[derive(Clone, Copy, Debug)]
pub enum ColorType {
    DeviceRgb([f32; 3]),
    DeviceGray(f32),
    DeviceCmyk([f32; 4])
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    color_type: ColorType,
    opacity: f32
}

impl Color {
    pub(crate) fn from_pdf(color_space: ColorSpace, c: &ColorComponents, opacity: f32) -> Self {
        let c_type = match color_space {
            ColorSpace::DeviceCmyk => ColorType::DeviceCmyk([c[0], c[1], c[2], c[3]]),
            ColorSpace::DeviceGray => ColorType::DeviceGray(c[0]),
            ColorSpace::DeviceRgb => ColorType::DeviceRgb([c[0], c[1], c[2]]),
        };
        
        Self {
            color_type: c_type,
            opacity,
        }
    }
    
    pub fn to_rgba(&self) -> AlphaColor<Srgb> {
        // Conversions according to section 10.4 in the spec.
        match self.color_type {
            ColorType::DeviceRgb(r) => {
                AlphaColor::new([r[0], r[1], r[2], self.opacity])
            }
            ColorType::DeviceGray(g) => {
                AlphaColor::new([g, g, g, self.opacity])
            }
            ColorType::DeviceCmyk(c) => {
                let red = 1.0 - 1.0f32.min(c[0] + c[3]);
                let green = 1.0 - 1.0f32.min(c[1] + c[3]);
                let blue = 1.0 - 1.0f32.min(c[2] + c[3]);
                
                AlphaColor::new([red, green, blue, self.opacity])
            }
        }
    }
}