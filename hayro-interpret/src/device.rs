use crate::{FillProps, StrokeProps};
use kurbo::{Affine, BezPath};
use peniko::color::{AlphaColor, Srgb};

pub trait Device {
    fn set_transform(&mut self, affine: Affine);
    fn set_paint(&mut self, color: AlphaColor<Srgb>);
    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps);
    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps);
}
