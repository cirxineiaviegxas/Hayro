use crate::color::Color;
use crate::{FillProps, StrokeProps};
use kurbo::{Affine, BezPath};
use peniko::Fill;
use skrifa::raw::tables::colr::Clip;

#[derive(Debug, Clone)]
pub struct ClipPath {
    pub path: BezPath,
    pub fill: Fill,
}

#[derive(Debug, Clone)]
pub struct Mask {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub trait Device {
    fn set_transform(&mut self, affine: Affine);
    fn set_paint(&mut self, color: Color);
    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps);
    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps);
    fn push_layer(&mut self, clip_path: Option<&ClipPath>, opacity: f32);
    fn apply_mask(&mut self, mask: &Mask);
    fn draw_rgba_image(&mut self, image_data: Vec<u8>, width: u32, height: u32);
    fn draw_stencil_image(&mut self, image_data: Vec<u8>, width: u32, height: u32);
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
        clip: Option<ClipPath>,
        opacity: f32,
    },
    ApplyMask {
        mask: Mask,
    },
    PopClip,
}
