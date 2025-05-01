use crate::font::blob::OpenTypeFontBlob;
use crate::font::encoding::{win_ansi, MAC_EXPERT, MAC_ROMAN};
use crate::font::standard::{StandardFont, select_standard_font};
use crate::font::true_type::read_encoding;
use crate::font::{Encoding, OutlinePath, UNITS_PER_EM};
use crate::util::OptionLog;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_ENCODING, BASE_FONT, DIFFERENCES, ENCODING};
use hayro_syntax::object::name::Name;
use kurbo::BezPath;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct Type1Font {
    base_font: Option<StandardFont>,
    blob: OpenTypeFontBlob,
    encoding: Encoding,
    encodings: HashMap<u8, String>,
}

impl Type1Font {
    pub fn new(dict: &Dict) -> Type1Font {
        let base_font = select_standard_font(dict)
            .warn_none("embedded type 1 fonts not supported yet")
            .unwrap_or(StandardFont::Courier);
        let blob = base_font.get_blob();

        let (encoding, encoding_map) = read_encoding(dict);

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
            bf.name_to_unicode(entry.as_str())
        } else {
            match self.encoding {
                Encoding::Standard => bf.code_to_unicode(code),
                Encoding::MacRoman => MAC_ROMAN.get(&code).and_then(|v| bf.name_to_unicode(v)),
                Encoding::WinAnsi => win_ansi::get(code).and_then(|v| bf.name_to_unicode(v)),
                Encoding::MacExpert => MAC_EXPERT.get(&code).and_then(|v| bf.name_to_unicode(v)),
                Encoding::BuiltIn => bf.code_to_unicode(code),
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

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        self.blob.outline_glyph(glyph)
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        self.blob
            .glyph_metrics()
            .advance_width(self.map_code(code))
            .unwrap_or(0.0)
    }
}
