use crate::font::Encoding;
use crate::font::blob::{CffFontBlob, Type1FontBlob};
use crate::font::encoding::{MAC_EXPERT, MAC_ROMAN, STANDARD, win_ansi};
use crate::font::standard::{StandardFont, select_standard_font};
use crate::font::true_type::{read_encoding, read_widths};
use crate::util::OptionLog;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{FONT_DESCRIPTOR, FONT_FILE, FONT_FILE3};
use hayro_syntax::object::stream::Stream;
use kurbo::BezPath;
use skrifa::raw::tables::glyf::Glyph;
use skrifa::{GlyphId, MetadataProvider};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct Type1Font(Kind);

impl Type1Font {
    pub fn new(dict: &Dict) -> Self {
        if is_cff(dict) {
            Self(Kind::Cff(Cff::new(dict)))
        } else if is_type1(dict) {
            Self(Kind::Type1(Type1::new(dict)))
        } else {
            Self(Kind::Standard(Standard::new(dict)))
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        match &self.0 {
            Kind::Standard(s) => s.map_code(code),
            Kind::Type1(s) => s.map_code(code),
            Kind::Cff(c) => c.map_code(code),
        }
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        match &self.0 {
            Kind::Standard(s) => s.outline_glyph(glyph),
            Kind::Cff(c) => c.outline_glyph(glyph),
            Kind::Type1(t) => t.outline_glyph(glyph),
        }
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        match &self.0 {
            Kind::Standard(s) => s.glyph_width(code),
            Kind::Cff(c) => c.glyph_width(code),
            Kind::Type1(t) => t.glyph_width(code),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Standard(Standard),
    Cff(Cff),
    Type1(Type1),
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
            .warn_none("couldnt find appropriate font")
            .unwrap_or(StandardFont::Courier);

        let (encoding, encoding_map) = read_encoding(dict);

        Self {
            base_font,
            encodings: encoding_map,
            encoding,
        }
    }

    fn code_to_ps_name(&self, code: u8) -> Option<&str> {
        let bf = self.base_font;

        self.encodings
            .get(&code)
            .map(String::as_str)
            .or_else(|| match self.encoding {
                Encoding::Standard => STANDARD.get(&code).copied(),
                Encoding::MacRoman => MAC_ROMAN.get(&code).copied(),
                Encoding::WinAnsi => win_ansi::get(code),
                Encoding::MacExpert => MAC_EXPERT.get(&code).copied(),
                Encoding::BuiltIn => bf.code_to_name(code),
            })
    }

    fn ps_name_to_unicode(&self, name: &str) -> Option<&str> {
        self.base_font
            .name_to_unicode(name)
            .warn_none(&format!("failed to map code {name} to a ps string."))
    }

    fn unicode_to_glyph(&self, name: &str) -> Option<GlyphId> {
        self.base_font
            .get_blob()
            .font_ref()
            .charmap()
            .map(name.chars().nth(0).unwrap())
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        self.code_to_ps_name(code)
            .and_then(|c| self.ps_name_to_unicode(c))
            .and_then(|n| self.unicode_to_glyph(n))
            .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        self.base_font.get_blob().outline_glyph(glyph)
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        self.code_to_ps_name(code)
            .and_then(|c| self.base_font.get_width(c))
            .unwrap()
    }
}

fn is_cff(dict: &Dict) -> bool {
    dict.get::<Dict>(FONT_DESCRIPTOR)
        .map(|dict| dict.contains_key(FONT_FILE3))
        .unwrap_or(false)
}

fn is_type1(dict: &Dict) -> bool {
    dict.get::<Dict>(FONT_DESCRIPTOR)
        .map(|dict| dict.contains_key(FONT_FILE))
        .unwrap_or(false)
}

#[derive(Debug)]
struct Type1 {
    font: Type1FontBlob,
    encoding: Encoding,
    widths: Vec<f32>,
    encodings: HashMap<u8, String>,
    // We simulate that Type1 glyphs have glyph IDs so we can handle them transparently
    // to OpenType fonts.
    glyph_to_string: RefCell<HashMap<GlyphId, String>>,
    string_to_glyph: RefCell<HashMap<String, GlyphId>>,
    glyph_counter: RefCell<u32>,
}

impl Type1 {
    pub fn new(dict: &Dict) -> Self {
        let descriptor = dict.get::<Dict>(FONT_DESCRIPTOR).unwrap();
        let data = descriptor.get::<Stream>(FONT_FILE).unwrap();
        let font = Type1FontBlob::new(Arc::new(data.decoded().unwrap().to_vec()));

        let (encoding, encodings) = read_encoding(dict);
        let widths = read_widths(dict, &descriptor);

        let mut glyph_to_string = HashMap::new();
        glyph_to_string.insert(GlyphId::NOTDEF, "notdef".to_string());

        let mut string_to_glyph = HashMap::new();
        string_to_glyph.insert("notdef".to_string(), GlyphId::NOTDEF);

        Self {
            font,
            encoding,
            glyph_to_string: RefCell::new(glyph_to_string),
            string_to_glyph: RefCell::new(string_to_glyph),
            glyph_counter: RefCell::new(1),
            widths,
            encodings,
        }
    }

    fn string_to_glyph(&self, string: &str) -> GlyphId {
        if let Some(g) = self.string_to_glyph.borrow().get(string) {
            *g
        } else {
            let gid = GlyphId::new(*self.glyph_counter.borrow());
            self.string_to_glyph
                .borrow_mut()
                .insert(string.to_string(), gid);
            self.glyph_to_string
                .borrow_mut()
                .insert(gid, string.to_string());

            *self.glyph_counter.borrow_mut() += 1;

            gid
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        let table = self.font.table();

        let get_glyph = |entry: &str| self.string_to_glyph(entry);

        if let Some(entry) = self.encodings.get(&code) {
            Some(get_glyph(entry))
        } else {
            match self.encoding {
                Encoding::Standard => STANDARD.get(&code).map(|v| get_glyph(*v)),
                Encoding::MacRoman => MAC_ROMAN.get(&code).map(|v| get_glyph(*v)),
                Encoding::WinAnsi => win_ansi::get(code).map(|v| get_glyph(v)),
                Encoding::MacExpert => MAC_EXPERT.get(&code).map(|v| get_glyph(*v)),
                Encoding::BuiltIn => table.code_to_string(code).map(|g| get_glyph(g)),
            }
        }
        .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        self.font
            .outline_glyph(self.glyph_to_string.borrow().get(&glyph).unwrap())
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        *self.widths.get(code as usize).unwrap()
    }
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

        let get_glyph = |entry: &str| {
            table
                .glyph_index_by_name(entry)
                .map(|g| GlyphId::new(g.0 as u32))
        };

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
        }
        .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        self.font.outline_glyph(glyph)
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        *self.widths.get(code as usize).unwrap()
    }
}
