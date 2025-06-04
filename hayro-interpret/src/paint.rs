use crate::color::Color;
use crate::pattern::{Pattern, ShadingPattern};
use kurbo::Affine;

#[derive(Clone, Debug)]
pub enum PaintType<'a> {
    Color(Color),
    Pattern(Pattern<'a>),
}

#[derive(Clone, Debug)]
pub struct Paint<'a> {
    pub paint_transform: Affine,
    pub paint_type: PaintType<'a>,
}
