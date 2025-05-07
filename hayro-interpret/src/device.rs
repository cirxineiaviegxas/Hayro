use crate::color::Color;
use crate::{FillProps, StrokeProps};
use kurbo::{Affine, BezPath};
use peniko::Fill;

pub trait Device {
    fn set_transform(&mut self, affine: Affine);
    fn set_paint(&mut self, color: Color);
    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps);
    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps);
    fn push_layer(&mut self, clip: &BezPath, fill: Fill, opactity: u8);
    fn pop(&mut self);
}

pub(crate) enum ReplayInstruction {
    SetTransform {
        affine: Affine,
    },
    SetPaint {
        color: Color,
    },
    StrokePath {
        path: BezPath,
        stroke_props: StrokeProps,
    },
    FillPath {
        path: BezPath,
        fill_props: FillProps,
    },
    PushLayer {
        clip: BezPath,
        fill: Fill,
        opacity: u8,
    },
    PopClip,
}
