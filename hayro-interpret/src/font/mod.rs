use crate::context::Context;
use crate::font::cid::Type0Font;
use crate::font::encoding::{MAC_EXPERT, MAC_OS_ROMAN, MAC_ROMAN, STANDARD, win_ansi};
use crate::font::true_type::TrueTypeFont;
use crate::font::type1::Type1Font;
use crate::font::type3::Type3;
use crate::glyph::{Glyph, OutlineGlyph, Type3Glyph};
use hayro_font_parser::OutlineBuilder;
use hayro_syntax::document::page::Resources;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::SUBTYPE;
use hayro_syntax::object::dict::keys::*;
use hayro_syntax::object::name::Name;
use kurbo::{Affine, BezPath, Vec2};
use skrifa::GlyphId;
use skrifa::outline::OutlinePen;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

pub(crate) const UNITS_PER_EM: f32 = 1000.0;

mod blob;
mod cid;
pub(crate) mod encoding;
mod standard;
mod true_type;
mod type1;
pub(crate) mod type3;

#[derive(Clone, Debug)]
pub struct Font<'a>(FontType<'a>);

impl<'a> Font<'a> {
    pub fn new(dict: &Dict<'a>) -> Option<Self> {
        let f_type = match dict.get::<Name>(SUBTYPE)? {
            TYPE1 | MM_TYPE1 => FontType::Type1(Arc::new(Type1Font::new(dict)?)),
            TRUE_TYPE => TrueTypeFont::new(dict)
                .map(|t| Arc::new(t))
                .map(FontType::TrueType)
                .or_else(|| {
                    Type1Font::new(dict)
                        .map(|t| Arc::new(t))
                        .map(FontType::Type1)
                })?,
            TYPE0 => FontType::Type0(Arc::new(Type0Font::new(dict)?)),
            TYPE3 => FontType::Type3(Arc::new(Type3::new(dict))),
            f => {
                println!(
                    "unimplemented font type {:?}",
                    std::str::from_utf8(f.deref()).unwrap()
                );

                return None;
            }
        };

        Some(Self(f_type))
    }

    pub fn map_code(&self, code: u16) -> GlyphId {
        match &self.0 {
            FontType::Type1(f) => {
                debug_assert!(code <= u8::MAX as u16);

                f.map_code(code as u8)
            }
            FontType::TrueType(t) => {
                debug_assert!(code <= u8::MAX as u16);

                t.map_code(code as u8)
            }
            FontType::Type0(t) => t.map_code(code),
            FontType::Type3(t) => {
                debug_assert!(code <= u8::MAX as u16);

                t.map_code(code as u8)
            }
        }
    }

    pub fn get_glyph(
        &self,
        glyph: GlyphId,
        ctx: &mut Context<'a>,
        resources: &Resources<'a>,
        origin_displacement: Vec2,
    ) -> Glyph<'a> {
        let glyph_transform = ctx.get().text_state.full_transform()
            * Affine::scale(1.0 / UNITS_PER_EM as f64)
            * Affine::translate(origin_displacement);

        match &self.0 {
            FontType::Type1(t) => {
                let font = OutlineFont::Type1(t.clone());
                Glyph::Outline(OutlineGlyph {
                    id: glyph,
                    font,
                    glyph_transform,
                })
            }
            FontType::TrueType(t) => {
                let font = OutlineFont::TrueType(t.clone());
                Glyph::Outline(OutlineGlyph {
                    id: glyph,
                    font,
                    glyph_transform,
                })
            }
            FontType::Type0(t) => {
                let font = OutlineFont::Type0(t.clone());
                Glyph::Outline(OutlineGlyph {
                    id: glyph,
                    font,
                    glyph_transform,
                })
            }
            FontType::Type3(t) => {
                let shape_glyph = Type3Glyph {
                    font: t.clone(),
                    glyph_id: glyph,
                    state: ctx.get().clone(),
                    parent_resources: resources.clone(),
                    cache: ctx.object_cache.clone(),
                    xref: ctx.xref,
                    glyph_transform,
                };

                Glyph::Shape(shape_glyph)
            }
        }
    }

    pub fn code_advance(&self, code: u16) -> Vec2 {
        match &self.0 {
            FontType::Type1(t) => {
                debug_assert!(code <= u8::MAX as u16);

                Vec2::new(t.glyph_width(code as u8) as f64, 0.0)
            }
            FontType::TrueType(t) => {
                debug_assert!(code <= u8::MAX as u16);

                Vec2::new(t.glyph_width(code as u8) as f64, 0.0)
            }
            FontType::Type0(t) => t.code_advance(code),
            FontType::Type3(t) => {
                debug_assert!(code <= u8::MAX as u16);

                Vec2::new(t.glyph_width(code as u8) as f64, 0.0)
            }
        }
    }

    pub fn origin_displacement(&self, code: u16) -> Vec2 {
        match &self.0 {
            FontType::Type1(_) => Vec2::default(),
            FontType::TrueType(_) => Vec2::default(),
            FontType::Type0(t) => t.origin_displacement(code),
            FontType::Type3(_) => Vec2::default(),
        }
    }

    pub fn code_len(&self) -> usize {
        match &self.0 {
            FontType::Type1(_) => 1,
            FontType::TrueType(_) => 1,
            FontType::Type0(t) => t.code_len(),
            FontType::Type3(_) => 1,
        }
    }

    pub fn is_horizontal(&self) -> bool {
        match &self.0 {
            FontType::Type1(_) => true,
            FontType::TrueType(_) => true,
            FontType::Type0(t) => t.is_horizontal(),
            FontType::Type3(_) => true,
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

#[derive(Debug, Clone)]
pub(crate) enum OutlineFont {
    Type1(Arc<Type1Font>),
    TrueType(Arc<TrueTypeFont>),
    Type0(Arc<Type0Font>),
}

impl OutlineFont {
    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        match self {
            OutlineFont::Type1(t) => t.outline_glyph(glyph),
            OutlineFont::TrueType(t) => t.outline_glyph(glyph),
            OutlineFont::Type0(t) => t.outline_glyph(glyph),
        }
    }
}

#[derive(Clone, Debug)]
enum FontType<'a> {
    Type1(Arc<Type1Font>),
    TrueType(Arc<TrueTypeFont>),
    Type0(Arc<Type0Font>),
    Type3(Arc<Type3<'a>>),
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
