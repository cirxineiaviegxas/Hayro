use std::sync::Arc;
use kurbo::{Affine, BezPath, Rect};
use skrifa::GlyphId;
use hayro_syntax::xref::XRef;
use crate::cache::Cache;
use crate::device::Device;
use crate::font::{Font, OutlineFont, UNITS_PER_EM};
use crate::font::type3::Type3;
use crate::{FillProps, Paint, RgbaImage, StencilImage, StrokeProps};
use crate::clip_path::ClipPath;
use crate::context::Context;

#[derive(Clone, Debug)]
pub struct OutlineGlyph {
    id: GlyphId,
    font: Arc<OutlineFont>
}

impl OutlineGlyph {
    pub fn outline(&self, font_size: f32) -> BezPath {
        Affine::scale((font_size / UNITS_PER_EM) as f64) * self.font.outline_glyph(self.id)
    }
}

pub struct ShapeGlyph<'a> {
    pub(crate) font: Type3<'a>,
    pub(crate) glyph_id: GlyphId,
    pub(crate) cache: Cache,
    pub(crate) xref: &'a XRef
}

impl<'a> ShapeGlyph<'a> {
    // pub fn interpret(&self, device: &mut impl Device, initial_transform: Affine) {
    //     let t = self.font.matrix * Affine::scale(UNITS_PER_EM as f64);
    //     // TODO: bbox?
    //     let mut context = Context::new_with(
    //         initial_transform * t,
    //         Rect::new(0.0, 0.0, 1.0, 1.0),
    //         self.cache.clone(),
    //         self.xref
    //     )
    // }
}

struct Type3ShapeGlyphDevice<'a, T: Device>(&'a mut T);

impl<'a, T: Device> Type3ShapeGlyphDevice<'a, T> {
    pub fn new(device: &'a mut T, initial_paint: Paint) -> Self {
        device.set_paint(initial_paint);
        
        Self(device)
    }
}

impl<T: Device> Device for Type3ShapeGlyphDevice<'_, T> {
    fn set_transform(&mut self, affine: Affine) {
        self.0.set_transform(affine);
    }

    fn set_paint_transform(&mut self, affine: Affine) {
        
    }

    fn set_paint(&mut self, paint: Paint) {
        
    }

    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps) {
       self.0.stroke_path(path, stroke_props)
    }

    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps) {
        self.0.fill_path(path, fill_props)
    }

    fn push_layer(&mut self, clip_path: Option<&ClipPath>, opacity: f32) {
       self.0.push_layer(clip_path, opacity)
    }

    fn draw_rgba_image(&mut self, image: RgbaImage) {
        
    }

    fn draw_stencil_image(&mut self, stencil: StencilImage) {
        self.0.draw_stencil_image(stencil);
    }

    fn pop(&mut self) {
       self.0.pop()
    }
}