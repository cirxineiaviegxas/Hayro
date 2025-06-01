use crate::color::Color;
use crate::pattern::ShadingPattern;

#[derive(Clone, Debug)]
pub enum Paint {
    Color(Color),
    Shading(ShadingPattern),
}
