use crate::font::encoding::{win_ansi, MAC_EXPERT, MAC_ROMAN, STANDARD};
use crate::font::standard::{StandardFont, select_standard_font};
use crate::font::true_type::{read_encoding, read_widths};
use crate::font::Encoding;
use crate::util::OptionLog;
use hayro_syntax::object::dict::Dict;
use kurbo::BezPath;
use skrifa::{GlyphId, MetadataProvider};
use std::collections::HashMap;
use std::sync::Arc;
use hayro_syntax::object::dict::keys::{FONT_DESCRIPTOR, FONT_FILE3};
use hayro_syntax::object::stream::Stream;
use crate::font::blob::CffFontBlob;

#[derive(Debug)]
pub(crate) struct Type1Font(Kind);

impl Type1Font {
    pub fn new(dict: &Dict) -> Self {
        if is_cff(dict) {
            Self(Kind::Cff(Cff::new(dict)))
        }   else {
            Self(Kind::Standard(Standard::new(dict)))
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        match &self.0 {
            Kind::Standard(s) => s.map_code(code),
            Kind::Cff(c) => c.map_code(code),
        }
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        match &self.0 {
            Kind::Standard(s) => s.outline_glyph(glyph),
            Kind::Cff(c) => c.outline_glyph(glyph),
        }
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        match &self.0 {
            Kind::Standard(s) => s.glyph_width(code),
            Kind::Cff(c) => c.glyph_width(code),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Standard(Standard),
    Cff(Cff),
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

fn is_cff(dict: &Dict) -> bool {
    dict.get::<Dict>(FONT_DESCRIPTOR)
        .map(|dict| dict.contains_key(FONT_FILE3))
        .unwrap_or(false)
}

#[derive(Debug)]
struct Cff {
    font: CffFontBlob,
    encoding: Encoding,
    widths: Vec<f32>,
    encodings: HashMap<u8, String>,
}

impl Cff {
    pub fn new(dict: &Dict) -> Self {
        let descriptor = dict.get::<Dict>(FONT_DESCRIPTOR).unwrap();
        let data = descriptor.get::<Stream>(FONT_FILE3).unwrap();
        let font = CffFontBlob::new(Arc::new(data.decoded().unwrap().to_vec()));
        
        let (encoding, encodings) = read_encoding(dict);
        let widths = read_widths(dict, &descriptor);
        
        Self {
            font,
            encoding,
            widths,
            encodings,
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        let table = self.font.table();
        
        let get_glyph = |entry: &str| table.glyph_index_by_name(entry).map(|g| GlyphId::new(g.0 as u32));

        if let Some(entry) = self.encodings.get(&code) {
            get_glyph(entry)
        } else {
            match self.encoding {
                Encoding::Standard => STANDARD.get(&code).and_then(|v| get_glyph(*v)),
                Encoding::MacRoman => MAC_ROMAN.get(&code).and_then(|v| get_glyph(*v)),
                Encoding::WinAnsi => win_ansi::get(code).and_then(|v| get_glyph(v)),
                Encoding::MacExpert => MAC_EXPERT.get(&code).and_then(|v| get_glyph(*v)),
                Encoding::BuiltIn => table.glyph_index(code).map(|g| GlyphId::new(g.0 as u32)),
            }
        }.unwrap_or(GlyphId::NOTDEF)
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        self.font.outline_glyph(glyph)
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        *self.widths.get(code as usize).unwrap()
    }
}
