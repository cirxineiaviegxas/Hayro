use crate::font::true_type::{read_encoding, read_widths};
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::FONT_MATRIX;
use hayro_syntax::object::stream::Stream;
use kurbo::Affine;
use skrifa::GlyphId;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug)]
struct Type3<'a> {
    widths: Vec<f32>,
    encodings: HashMap<u8, String>,
    dict: Dict<'a>,
    // TODO: Don't automatically resolve glyph streams?
    glyph_programs: Vec<Stream<'a>>,
    // Similarly to Type1 fonts, we simulate that Type3 glyphs have glyph IDs
    // so we can handle them transparently to OpenType fonts.
    glyph_to_string: RefCell<HashMap<GlyphId, String>>,
    string_to_glyph: RefCell<HashMap<String, GlyphId>>,
    glyph_counter: RefCell<u32>,
    matrix: Affine,
}

impl<'a> Type3<'a> {
    pub fn new(dict: &Dict<'a>) -> Self {
        let (_, encodings) = read_encoding(dict);
        let widths = read_widths(&dict, &dict);

        let matrix = Affine::new(
            dict.get::<[f64; 6]>(FONT_MATRIX)
                .unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]),
        );

        let mut glyph_to_string = HashMap::new();
        let mut string_to_glyph = HashMap::new();

        Self {
            glyph_to_string: RefCell::new(glyph_to_string),
            string_to_glyph: RefCell::new(string_to_glyph),
            glyph_counter: RefCell::new(1),
            widths,
            encodings,
            matrix,
            dict: dict.clone(),
            glyph_programs: vec![],
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
        self.encodings
            .get(&code)
            .map(|g| self.string_to_glyph(g))
            .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        *self.widths.get(code as usize).unwrap()
    }
}
