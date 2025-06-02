use crate::cache::Cache;
use crate::clip_path::ClipPath;
use crate::device::Device;
use crate::font::type3::Type3;
use crate::font::{Font, OutlineFont, UNITS_PER_EM};
use crate::interpret::state::State;
use crate::{FillProps, Paint, RgbaImage, StencilImage, StrokeProps};
use hayro_syntax::document::page::Resources;
use hayro_syntax::xref::XRef;
use kurbo::{Affine, BezPath, Rect};
use skrifa::GlyphId;
use std::sync::Arc;

pub enum Glyph<'a> {
    Outline(OutlineGlyph),
    Shape(Type3Glyph<'a>),
}

impl Glyph<'_> {
    pub fn glyph_transform(&self) -> Affine {
        match self {
            Glyph::Outline(o) => o.glyph_transform,
            Glyph::Shape(s) => s.glyph_transform,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OutlineGlyph {
    pub(crate) id: GlyphId,
    pub(crate) font: OutlineFont,
    pub glyph_transform: Affine,
}

impl OutlineGlyph {
    pub fn outline(&self) -> BezPath {
        self.font.outline_glyph(self.id)
    }
}

pub struct Type3Glyph<'a> {
    pub(crate) font: Arc<Type3<'a>>,
    pub(crate) glyph_id: GlyphId,
    pub(crate) state: State<'a>,
    pub(crate) parent_resources: Resources<'a>,
    pub(crate) cache: Cache,
    pub(crate) glyph_transform: Affine,
    pub(crate) xref: &'a XRef,
}

impl<'a> Type3Glyph<'a> {
    pub fn interpret(&self, device: &mut impl Device, paint: &Paint) {
        self.font.render_glyph(&self, paint, device);
    }
}
