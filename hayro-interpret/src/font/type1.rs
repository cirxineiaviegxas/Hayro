use std::collections::HashMap;
use kurbo::BezPath;
use skrifa::{GlyphId, MetadataProvider};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::DrawSettings;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_ENCODING, BASE_FONT, DIFFERENCES, ENCODING};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::Object;
use crate::font::blob::FontBlob;
use crate::font::{Encoding, OutlinePath};
use crate::font::encoding::{MAC_EXPERT, MAC_ROMAN, WIN_ANSI};
use crate::font::standard::{select, StandardFont};
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

        let mut encoding_map = HashMap::new();
        let encoding = read_encoding(dict, &mut encoding_map);

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

pub(crate) fn read_encoding(dict: &Dict, encoding_map: &mut HashMap<u8, String>) -> Encoding {
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

        get_encoding_base(&encoding_dict, BASE_ENCODING)
    } else {
        get_encoding_base(&dict, ENCODING)
    }
    
}