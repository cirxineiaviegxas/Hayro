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

mod standard;
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
    base_font: Option<StandardFont>,
    blob: FontBlob,
    encoding: Encoding,
    encodings: HashMap<u8, String>,
}

impl Type1Font {
    pub fn new(dict: &Dict) -> Type1Font {
        let base_font = dict.get::<Name>(BASE_FONT)
            .and_then(|b| select(b)).unwrap();
        let blob = base_font.get_blob();

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
