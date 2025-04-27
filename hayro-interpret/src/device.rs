use crate::{FillProps, StrokeProps};
use kurbo::{Affine, BezPath};
use peniko::Fill;
use crate::color::Color;

pub trait Device {
    fn set_transform(&mut self, affine: Affine);
    fn set_paint(&mut self, color: Color);
    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps);
    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps);
    fn push_clip(&mut self, clip: &BezPath, fill: Fill);
    fn pop_clip(&mut self);
}
