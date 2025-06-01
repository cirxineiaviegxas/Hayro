use crate::color::Color;
use crate::pattern::ShadingPattern;

pub enum Paint {
    Color(Color),
    Shading(ShadingPattern),
}
