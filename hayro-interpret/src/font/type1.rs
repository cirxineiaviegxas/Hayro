use crate::font::encoding::{win_ansi, MAC_EXPERT, MAC_ROMAN, STANDARD};
use crate::font::standard::{StandardFont, select_standard_font};
use crate::font::true_type::read_encoding;
use crate::font::Encoding;
use crate::util::OptionLog;
use hayro_syntax::object::dict::Dict;
use kurbo::BezPath;
use skrifa::{GlyphId, MetadataProvider};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) enum Type1Font {
    Standard(Standard),
}

impl Type1Font {
    pub fn new(dict: &Dict) -> Self {
        Self::Standard(Standard::new(dict))
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        match self {
            Type1Font::Standard(s) => s.map_code(code),
        }
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        match self {
            Type1Font::Standard(s) => s.outline_glyph(glyph),
        }
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        match self {
            Type1Font::Standard(s) => s.glyph_width(code),
        }
    }
}

#[derive(Debug)]
struct Standard {
    base_font: StandardFont,
    encoding: Encoding,
    encodings: HashMap<u8, String>,
}

impl Standard {
    pub fn new(dict: &Dict) -> Standard {
        let base_font = select_standard_font(dict)
            .warn_none("embedded type 1 fonts not supported yet")
            .unwrap_or(StandardFont::Courier);

        let (encoding, encoding_map) = read_encoding(dict);

        Self {
            base_font,
            encodings: encoding_map,
            encoding,
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        let bf = self.base_font;
        let blob = bf.get_blob();

        let cp = if let Some(entry) = self.encodings.get(&code) {
            bf.name_to_unicode(entry.as_str())
        } else {
            match self.encoding {
                Encoding::Standard => STANDARD.get(&code).and_then(|v| bf.name_to_unicode(v)),
                Encoding::MacRoman => MAC_ROMAN.get(&code).and_then(|v| bf.name_to_unicode(v)),
                Encoding::WinAnsi => win_ansi::get(code).and_then(|v| bf.name_to_unicode(v)),
                Encoding::MacExpert => MAC_EXPERT.get(&code).and_then(|v| bf.name_to_unicode(v)),
                Encoding::BuiltIn => bf.code_to_unicode(code),
            }
        }
        .warn_none(&format!("failed to map code {code} to a ps string."));

        cp.and_then(|c| {
            blob
                .font_ref()
                .charmap()
                .map(c.chars().nth(0).unwrap())
        })
        .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        self.base_font.get_blob().outline_glyph(glyph)
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        self.base_font
            .get_blob()
            .glyph_metrics()
            .advance_width(self.map_code(code))
            .unwrap_or(0.0)
    }
}
