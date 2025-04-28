use crate::font::base::BaseFont;
use crate::font::blob::{
    COURIER_BOLD, COURIER_BOLD_ITALIC, COURIER_ITALIC, COURIER_REGULAR,
    ZAPF_DINGS_BAT, TIMES_BOLD, TIMES_ROMAN_BOLD_ITALIC, TIMES_ITALIC, TIMES_REGULAR,
    FontBlob, HELVETICA_BOLD, HELVETICA_BOLD_ITALIC, HELVETICA_ITALIC, HELVETICA_REGULAR, SYMBOL,
};
use crate::font::glyph_list::ZAPF_DINGS;
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_FONT, DIFFERENCES, ENCODING, SUBTYPE, TYPE};
use hayro_syntax::object::name::Name;
use kurbo::BezPath;
use skrifa::instance::LocationRef;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};
use std::collections::HashMap;
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
enum FontType {
    Type1Font(Type1Font),
}

#[derive(Debug)]
struct Type1Font {
    base_font: Option<BaseFont>,
    blob: FontBlob,
    encoding: HashMap<u8, String>,
}

impl Type1Font {
    pub fn new(dict: &Dict) -> Type1Font {
        let (base_font, blob) = if let Some(n) = dict.get::<Name>(BASE_FONT) {
            match n.get().as_ref() {
                b"Helvetica" => (BaseFont::Helvetica, HELVETICA_REGULAR.clone()),
                b"Helvetica-Bold" => (BaseFont::HelveticaBold, HELVETICA_BOLD.clone()),
                b"Helvetica-BoldOblique" => {
                    (BaseFont::HelveticaBoldOblique, HELVETICA_BOLD_ITALIC.clone())
                }
                b"Helvetica-Oblique" => (BaseFont::HelveticaOblique, HELVETICA_ITALIC.clone()),
                b"Courier" => (BaseFont::Courier, COURIER_REGULAR.clone()),
                b"Courier-Bold" => (BaseFont::CourierBold, COURIER_BOLD.clone()),
                b"Courier-BoldOblique" => (
                    BaseFont::CourierBoldOblique,
                    COURIER_BOLD_ITALIC.clone(),
                ),
                b"Courier-Oblique" => (BaseFont::CourierOblique, COURIER_ITALIC.clone()),
                b"Times-Roman" => (BaseFont::TimesRoman, TIMES_REGULAR.clone()),
                b"Times-Bold" => (BaseFont::TimesBold, TIMES_BOLD.clone()),
                b"Times-Italic" => (BaseFont::TimesItalic, TIMES_ITALIC.clone()),
                b"Times-BoldItalic" => (BaseFont::TimesBoldItalic, TIMES_ROMAN_BOLD_ITALIC.clone()),
                b"Symbol" => (BaseFont::Symbol, SYMBOL.clone()),
                b"ZapfDingbats" => (BaseFont::ZapfDingBats, ZAPF_DINGS_BAT.clone()),
                _ => unimplemented!(),
            }
        } else {
            unimplemented!()
        };

        let mut encoding_map = HashMap::new();

        if let Some(differences) = dict
            .get::<Dict>(ENCODING)
            .and_then(|d| d.get::<Array>(DIFFERENCES))
        {
            let mut entries = differences.iter::<Object>();

            let mut code = 0;

            while let Some(obj) = entries.next() {
                if let Ok(num) = obj.clone().cast::<i32>() {
                    code = num;
                }   else if let Ok(name) = obj.cast::<Name>() {
                    encoding_map.insert(code as u8, name.as_str());
                    code += 1;
                }
            }
        }

        Self {
            base_font: Some(base_font),
            encoding: encoding_map,
            blob,
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        let bf = self.base_font.as_ref().unwrap();
        let cp = if let Some(entry) = self.encoding.get(&code) {
            bf.ps_to_unicode(entry.as_str()).unwrap()
        } else {
            bf.map_code(code).unwrap()
        };
        self.blob
            .font_ref()
            .charmap()
            .map(cp.chars().nth(0).unwrap())
            .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn draw_glyph(&self, glyph: GlyphId) -> BezPath {
        let mut path = OutlinePath(BezPath::new());
        let draw_settings = DrawSettings::unhinted(Size::new(1.0), LocationRef::default());

        let Some(outline) = self.blob.outline_glyphs().get(glyph) else {
            return BezPath::new();
        };

        let _ = outline.draw(draw_settings, &mut path);
        path.0
    }

    pub fn glyph_width(&self, glyph: GlyphId) -> f32 {
        self.blob
            .glyph_metrics()
            .advance_width(glyph)
            .unwrap_or(0.0)
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
