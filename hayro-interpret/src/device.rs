use crate::{FillProps, StrokeProps};
use kurbo::{Affine, BezPath};
use crate::clip_path::ClipPath;
use crate::paint::Paint;

pub trait Device {
    fn set_transform(&mut self, affine: Affine);
    fn set_paint(&mut self, paint: Paint);
    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps);
    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps);
    fn push_layer(&mut self, clip_path: Option<&ClipPath>, opacity: f32);
    fn draw_rgba_image(
        &mut self,
        image_data: Vec<u8>,
        width: u32,
        height: u32,
        is_stencil: bool,
        interpolate: bool,
    );
    fn set_anti_aliasing(&mut self, val: bool);
    fn pop(&mut self);
}

pub(crate) enum ReplayInstruction {
    SetTransform {
        affine: Affine,
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
    DrawImage {
        image_data: Vec<u8>,
        width: u32,
        height: u32,
        is_stencil: bool,
        interpolate: bool,
    },
    AntiAliasing {
        val: bool,
    },
    PopClip,
}
