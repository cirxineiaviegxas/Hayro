use crate::clip_path::ClipPath;
use crate::image::{RgbaImage, StencilImage};
use crate::paint::Paint;
use crate::{FillProps, StrokeProps};
use kurbo::{Affine, BezPath};

pub trait Device {
    fn set_transform(&mut self, affine: Affine);
    fn set_paint_transform(&mut self, affine: Affine);
    fn set_paint(&mut self, paint: Paint);
    fn stroke_path(&mut self, path: &BezPath);
    fn set_stroke_properties(&mut self, stroke_props: &StrokeProps);
    fn fill_path(&mut self, path: &BezPath);
    fn set_fill_properties(&mut self, fill_props: &FillProps);
    fn push_layer(&mut self, clip_path: Option<&ClipPath>, opacity: f32);
    fn draw_rgba_image(&mut self, image: RgbaImage);
    fn draw_stencil_image(&mut self, stencil: StencilImage);
    fn pop(&mut self);
}

pub(crate) enum ReplayInstruction {
    SetTransform {
        affine: Affine,
    },
    SetPaintTransform {
        affine: Affine,
    },
    StrokePath {
        path: BezPath,
    },
    StrokeProperties {
        stroke_props: StrokeProps,
    },
    FillPath {
        path: BezPath,
    },
    FillProperties {
        fill_props: FillProps,
    },
    PushLayer {
        clip: Option<ClipPath>,
        opacity: f32,
    },
    DrawImage {
        image: RgbaImage,
    },
    DrawStencil {
        stencil_image: StencilImage,
    },
    PopClip,
}
