use crate::font::standard::{select, StandardFont};
use crate::font::encoding::{MAC_EXPERT, MAC_ROMAN, WIN_ANSI};
use crate::util::OptionLog;
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_ENCODING, BASE_FONT, DIFFERENCES, ENCODING, SUBTYPE};
use hayro_syntax::object::name::Name;
use kurbo::BezPath;
use skrifa::instance::LocationRef;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use crate::font::blob::FontBlob;
use crate::font::type1::Type1Font;

mod standard;
mod blob;
mod encoding;
mod type1;

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

    pub fn outline(&self, glyph: GlyphId) -> BezPath {
        match self.0.as_ref() {
            FontType::Type1Font(t) => t.draw_glyph(glyph),
        }
    }

    pub fn glyph_width(&self, glyph: GlyphId) -> f32 {
        match self.0.as_ref() {
            FontType::Type1Font(t) => t.glyph_width(glyph),
        }
    }
}

#[derive(Debug)]
enum Encoding {
    Standard,
    MacRoman,
    WinAnsi,
    MacExpert,
}

#[derive(Debug)]
enum FontType {
    Type1Font(Type1Font),
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
