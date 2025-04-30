use crate::font::base::BaseFont;
use crate::font::blob::{
    COURIER_BOLD, COURIER_BOLD_ITALIC, COURIER_ITALIC, COURIER_REGULAR, FontBlob, HELVETICA_BOLD,
    HELVETICA_BOLD_ITALIC, HELVETICA_ITALIC, HELVETICA_REGULAR, SYMBOL, TIMES_BOLD, TIMES_ITALIC,
    TIMES_REGULAR, TIMES_ROMAN_BOLD_ITALIC, ZAPF_DINGS_BAT,
};
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

mod base;
mod blob;
mod encoding;

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

#[derive(Debug)]
struct Type1Font {
    base_font: Option<BaseFont>,
    blob: FontBlob,
    encoding: Encoding,
    encodings: HashMap<u8, String>,
}

impl Type1Font {
    pub fn new(dict: &Dict) -> Type1Font {
        let (base_font, blob) = if let Some(n) = dict.get::<Name>(BASE_FONT) {
            // See <https://github.com/apache/pdfbox/blob/4438b8fdc67a3a9ebfb194595d0e81f88b708a37/pdfbox/src/main/java/org/apache/pdfbox/pdmodel/font/FontMapperImpl.java#L62-L102>
            match n.get().as_ref() {
                b"Helvetica" | b"ArialMT" | b"Arial" | b"LiberationSans" | b"NimbusSanL-Regu" => {
                    (BaseFont::Helvetica, HELVETICA_REGULAR.clone())
                }
                b"Helvetica-Bold"
                | b"Arial-BoldMT"
                | b"Arial-Bold"
                | b"LiberationSans-Bold"
                | b"NimbusSanL-Bold" => (BaseFont::HelveticaBold, HELVETICA_BOLD.clone()),
                b"Helvetica-Oblique"
                | b"Arial-ItalicMT"
                | b"Arial-Italic"
                | b"Helvetica-Italic"
                | b"LiberationSans-Italic"
                | b"NimbusSanL-ReguItal" => (BaseFont::HelveticaOblique, HELVETICA_ITALIC.clone()),
                b"Helvetica-BoldOblique"
                | b"Arial-BoldItalicMT"
                | b"Helvetica-BoldItalic"
                | b"LiberationSans-BoldItalic"
                | b"NimbusSanL-BoldItal" => (
                    BaseFont::HelveticaBoldOblique,
                    HELVETICA_BOLD_ITALIC.clone(),
                ),
                b"Courier" | b"CourierNew" | b"CourierNewPSMT" | b"LiberationMono"
                | b"NimbusMonL-Regu" => (BaseFont::Courier, COURIER_REGULAR.clone()),
                b"Courier-Bold"
                | b"CourierNewPS-BoldMT"
                | b"CourierNew-Bold"
                | b"LiberationMono-Bold"
                | b"NimbusMonL-Bold" => (BaseFont::CourierBold, COURIER_BOLD.clone()),
                b"Courier-Oblique"
                | b"CourierNewPS-ItalicMT"
                | b"CourierNew-Italic"
                | b"LiberationMono-Italic"
                | b"NimbusMonL-ReguObli" => (BaseFont::CourierOblique, COURIER_ITALIC.clone()),
                b"Courier-BoldOblique"
                | b"CourierNewPS-BoldItalicMT"
                | b"CourierNew-BoldItalic"
                | b"LiberationMono-BoldItalic"
                | b"NimbusMonL-BoldObli" => {
                    (BaseFont::CourierBoldOblique, COURIER_BOLD_ITALIC.clone())
                }
                b"Times-Roman"
                | b"TimesNewRomanPSMT"
                | b"TimesNewRoman"
                | b"TimesNewRomanPS"
                | b"LiberationSerif"
                | b"NimbusRomNo9L-Regu" => (BaseFont::TimesRoman, TIMES_REGULAR.clone()),
                b"Times-Bold"
                | b"TimesNewRomanPS-BoldMT"
                | b"TimesNewRomanPS-Bold"
                | b"TimesNewRoman-Bold"
                | b"LiberationSerif-Bold"
                | b"NimbusRomNo9L-Medi" => (BaseFont::TimesBold, TIMES_BOLD.clone()),
                b"Times-Italic"
                | b"TimesNewRomanPS-ItalicMT"
                | b"TimesNewRomanPS-Italic"
                | b"TimesNewRoman-Italic"
                | b"LiberationSerif-Italic"
                | b"NimbusRomNo9L-ReguItal" => (BaseFont::TimesItalic, TIMES_ITALIC.clone()),
                b"Times-BoldItalic"
                | b"TimesNewRomanPS-BoldItalicMT"
                | b"TimesNewRomanPS-BoldItalic"
                | b"TimesNewRoman-BoldItalic"
                | b"LiberationSerif-BoldItalic"
                | b"NimbusRomNo9L-MediItal" => {
                    (BaseFont::TimesBoldItalic, TIMES_ROMAN_BOLD_ITALIC.clone())
                }
                b"Symbol" | b"SymbolMT" | b"StandardSymL" => (BaseFont::Symbol, SYMBOL.clone()),
                b"ZapfDingbats"
                | b"ZapfDingbatsITCbyBT-Regular"
                | b"ZapfDingbatsITC"
                | b"Dingbats"
                | b"MS-Gothic" => (BaseFont::ZapfDingBats, ZAPF_DINGS_BAT.clone()),

                _ => unimplemented!(),
            }
        } else {
            unimplemented!()
        };

        let mut encoding_map = HashMap::new();

        fn get_encoding_base(dict: &Dict, name: Name) -> Encoding {
            match dict.get::<Name>(name) {
                Some(n) => match n.get().as_ref() {
                    b"WinAnsiEncoding" => Encoding::WinAnsi,
                    b"MacRomanEncoding" => Encoding::MacRoman,
                    b"MacExpertEncoding" => Encoding::MacExpert,
                    _ => Encoding::Standard,
                },
                None => Encoding::Standard,
            }
        }

        let encoding;

        if let Some(encoding_dict) = dict.get::<Dict>(ENCODING) {
            if let Some(differences) = encoding_dict.get::<Array>(DIFFERENCES) {
                let mut entries = differences.iter::<Object>();

                let mut code = 0;

                while let Some(obj) = entries.next() {
                    if let Ok(num) = obj.clone().cast::<i32>() {
                        code = num;
                    } else if let Ok(name) = obj.cast::<Name>() {
                        encoding_map.insert(code as u8, name.as_str());
                        code += 1;
                    }
                }
            }

            encoding = get_encoding_base(&encoding_dict, BASE_ENCODING);
        } else {
            encoding = get_encoding_base(&dict, ENCODING);
        }

        Self {
            base_font: Some(base_font),
            encodings: encoding_map,
            encoding,
            blob,
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        let bf = self.base_font.as_ref().unwrap();

        let cp = if let Some(entry) = self.encodings.get(&code) {
            bf.ps_to_unicode(entry.as_str())
        } else {
            match self.encoding {
                Encoding::Standard => bf.map_code(code),
                Encoding::MacRoman => MAC_ROMAN.get(&code).and_then(|v| bf.ps_to_unicode(v)),
                Encoding::WinAnsi => WIN_ANSI.get(&code).and_then(|v| bf.ps_to_unicode(v)),
                Encoding::MacExpert => MAC_EXPERT.get(&code).and_then(|v| bf.ps_to_unicode(v)),
            }
        }
        .warn_none(&format!("failed to map code {code} to a ps string."));

        cp.and_then(|c| {
            self.blob
                .font_ref()
                .charmap()
                .map(c.chars().nth(0).unwrap())
        })
        .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn draw_glyph(&self, glyph: GlyphId) -> BezPath {
        let mut path = OutlinePath(BezPath::new());
        let draw_settings = DrawSettings::unhinted(Size::new(1000.0), LocationRef::default());

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
