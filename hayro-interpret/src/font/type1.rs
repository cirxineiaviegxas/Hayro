use std::collections::HashMap;
use kurbo::BezPath;
use skrifa::{GlyphId, MetadataProvider};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::DrawSettings;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_ENCODING, BASE_FONT, DIFFERENCES, ENCODING};
use hayro_syntax::object::name::Name;
use crate::font::blob::FontBlob;
use crate::font::{Encoding, OutlinePath};
use crate::font::encoding::{MAC_EXPERT, MAC_ROMAN, WIN_ANSI};
use crate::font::standard::{select, StandardFont};
use crate::font::true_type::read_encoding;
use crate::util::OptionLog;

#[derive(Debug)]
pub(crate) struct Type1Font {
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
                Encoding::WinAnsi => WIN_ANSI.get(&code).and_then(|v| bf.name_to_unicode(v)),
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