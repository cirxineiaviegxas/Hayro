use crate::font::cid::Type0Font;
use crate::font::encoding::{MAC_EXPERT, MAC_OS_ROMAN, MAC_ROMAN, STANDARD, win_ansi};
use crate::font::true_type::TrueTypeFont;
use crate::font::type1::Type1Font;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::SUBTYPE;
use hayro_syntax::object::name::Name;
use hayro_syntax::object::name::names::*;
use kurbo::{BezPath, Vec2};
use pdf_font_parser::OutlineBuilder;
use skrifa::GlyphId;
use skrifa::outline::OutlinePen;
use std::fmt::Debug;
use std::sync::Arc;

pub(crate) const UNITS_PER_EM: f32 = 1000.0;

mod blob;
mod cid;
pub(crate) mod encoding;
mod standard;
mod true_type;
mod type1;

#[derive(Clone, Debug)]
pub struct Font(Arc<FontType>);

impl Font {
    pub fn new(dict: &Dict) -> Option<Self> {
        let f_type = match dict.get::<Name>(SUBTYPE)?.as_ref() {
            TYPE1 => FontType::Type1(Type1Font::new(dict)?),
            TRUE_TYPE => FontType::TrueType(TrueTypeFont::new(dict)?),
            TYPE0 => FontType::Type0(Type0Font::new(dict)?),
            _ => unimplemented!(),
        };

        Some(Self(Arc::new(f_type)))
    }

    pub fn map_code(&self, code: u16) -> GlyphId {
        match self.0.as_ref() {
            FontType::Type1(f) => {
                debug_assert!(code <= u8::MAX as u16);

                f.map_code(code as u8)
            }
            FontType::TrueType(t) => {
                debug_assert!(code <= u8::MAX as u16);

                t.map_code(code as u8)
            }
            FontType::Type0(t) => t.map_code(code),
        }
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        match self.0.as_ref() {
            FontType::Type1(t) => t.outline_glyph(glyph),
            FontType::TrueType(t) => t.outline_glyph(glyph),
            FontType::Type0(t) => t.outline_glyph(glyph),
        }
    }

    pub fn code_advance(&self, code: u16) -> Vec2 {
        match self.0.as_ref() {
            FontType::Type1(t) => {
                debug_assert!(code <= u8::MAX as u16);

                Vec2::new(t.glyph_width(code as u8) as f64, 0.0)
            }
            FontType::TrueType(t) => {
                debug_assert!(code <= u8::MAX as u16);

                Vec2::new(t.glyph_width(code as u8) as f64, 0.0)
            }
            FontType::Type0(t) => t.code_advance(code),
        }
    }

    pub fn code_len(&self) -> usize {
        match self.0.as_ref() {
            FontType::Type1(_) => 1,
            FontType::TrueType(_) => 1,
            FontType::Type0(t) => t.code_len(),
        }
    }

    pub fn is_horizontal(&self) -> bool {
        match self.0.as_ref() {
            FontType::Type1(_) => true,
            FontType::TrueType(_) => true,
            FontType::Type0(t) => t.is_horizontal(),
        }
    }
}

#[derive(Debug)]
enum Encoding {
    Standard,
    MacRoman,
    WinAnsi,
    MacExpert,
    BuiltIn,
}

impl Encoding {
    fn lookup(&self, code: u8) -> Option<&'static str> {
        match self {
            Encoding::Standard => STANDARD.get(&code).copied(),
            Encoding::MacRoman => MAC_ROMAN
                .get(&code)
                .copied()
                .or_else(|| MAC_OS_ROMAN.get(&code).copied()),
            Encoding::WinAnsi => win_ansi::get(code),
            Encoding::MacExpert => MAC_EXPERT.get(&code).copied(),
            Encoding::BuiltIn => None,
        }
    }
}

#[derive(Debug)]
enum FontType {
    Type1(Type1Font),
    TrueType(TrueTypeFont),
    Type0(Type0Font),
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

impl OutlineBuilder for OutlinePath {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to((x1, y1), (x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to((x1, y1), (x2, y2), (x, y));
    }

    fn close(&mut self) {
        self.0.close_path();
    }
}
