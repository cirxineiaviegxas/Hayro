use crate::color::Color;
use crate::pattern::ShadingPattern;
use kurbo::Affine;

#[derive(Clone, Debug)]
pub enum PaintType {
    Color(Color),
    Shading(ShadingPattern),
}

#[derive(Clone, Debug)]
pub struct Paint {
    pub paint_transform: Affine,
    pub paint_type: PaintType,
}
