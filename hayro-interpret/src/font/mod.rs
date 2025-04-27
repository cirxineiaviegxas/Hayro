use crate::font::base::BaseFont;
use crate::font::blob::{FontBlob, ROBOTO};
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_FONT, SUBTYPE, TYPE};
use hayro_syntax::object::name::Name;
use kurbo::BezPath;
use skrifa::instance::LocationRef;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};
use std::fmt::Debug;
use std::sync::Arc;

mod base;
mod blob;
mod encodings;
mod glyph_list;

#[derive(Clone, Debug)]
pub struct Font(Arc<FontType>);

impl Font {
    pub fn new(dict: &Dict) -> Option<Self> {
        let f_type = match dict.get::<Name>(SUBTYPE)?.as_str().as_bytes() {
            b"Type1" => FontType::Type1Font(Type1Font::new(dict)),
            _ => unimplemented!(),
        };

        Some(Self(Arc::new(f_type)))
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        match self.0.as_ref() {
            FontType::Type1Font(f) => f.map_code(code),
        }
    }

    pub fn outline(&self, size: f32, glyph: GlyphId) -> BezPath {
        match self.0.as_ref() {
            FontType::Type1Font(t) => t.draw_glyph(size, glyph),
        }
    }
}

#[derive(Debug)]
enum FontType {
    Type1Font(Type1Font),
}

#[derive(Debug)]
struct Type1Font {
    base_font: Option<BaseFont>,
    blob: FontBlob,
}

impl Type1Font {
    pub fn new(dict: &Dict) -> Type1Font {
        let (base_font, blob) = if let Some(n) = dict.get::<Name>(BASE_FONT) {
            match n.get().as_ref() {
                b"Helvetica" => (BaseFont::Helvetica, ROBOTO.clone()),
                _ => unimplemented!(),
            }
        } else {
            unimplemented!()
        };

        Self {
            base_font: Some(base_font),
            blob,
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        let bf = self.base_font.as_ref().unwrap();
        let cp = bf.map_code(code).unwrap();
        self.blob
            .font_ref()
            .charmap()
            .map(cp.chars().nth(0).unwrap())
            .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn draw_glyph(&self, size: f32, glyph: GlyphId) -> BezPath {
        let mut path = OutlinePath(BezPath::new());
        let draw_settings = DrawSettings::unhinted(Size::new(size), LocationRef::default());

        let Some(outline) = self.blob.outline_glyphs().get(glyph) else {
            return BezPath::new();
        };

        let _ = outline.draw(draw_settings, &mut path);
        path.0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TextRenderingMode {
    #[default]
    Fill,
    Stroke,
    FillStroke,
    Invisible,
    FillAndClip,
    StrokeAndClip,
    FillAndStrokeAndClip,
    Clip,
}

struct OutlinePath(BezPath);

// Note that we flip the y-axis to match our coordinate system.
impl OutlinePen for OutlinePath {
    #[inline]
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((x, y));
    }

    #[inline]
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x, y));
    }

    #[inline]
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.curve_to((cx0, cy0), (cx1, cy1), (x, y));
    }

    #[inline]
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.quad_to((cx, cy), (x, y));
    }

    #[inline]
    fn close(&mut self) {
        self.0.close_path();
    }
}
