use crate::color::Color;
use crate::context::Context;
use crate::device::{Device, ReplayInstruction};
use crate::font::true_type::{read_encoding, read_widths};
use crate::{FillProps, StrokeProps, interpret};
use hayro_syntax::content::{TypedIter, UntypedIter};
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{CHAR_PROCS, FONT_MATRIX, RESOURCES};
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, BezPath};
use peniko::Fill;
use skrifa::GlyphId;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Type3GlyphDescription(pub(crate) Vec<ReplayInstruction>, pub(crate) Affine);

impl Type3GlyphDescription {
    pub fn new(affine: Affine) -> Self {
        Type3GlyphDescription(Vec::new(), affine)
    }
}

impl Device for Type3GlyphDescription {
    fn set_transform(&mut self, affine: Affine) {
        self.0.push(ReplayInstruction::SetTransform { affine });
    }

    fn set_paint(&mut self, _: Color) {}

    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps) {
        self.0.push(ReplayInstruction::StrokePath {
            path: path.clone(),
            stroke_props: stroke_props.clone(),
        })
    }

    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps) {
        self.0.push(ReplayInstruction::FillPath {
            path: path.clone(),
            fill_props: fill_props.clone(),
        })
    }

    fn push_layer(&mut self, clip: &BezPath, fill: Fill, opacity: u8) {
        self.0.push(ReplayInstruction::PushLayer {
            clip: clip.clone(),
            fill,
            opacity,
        });
    }

    fn draw_rgba_image(&mut self, _: Vec<u8>, _: u32, _: u32) {}

    fn pop(&mut self) {
        self.0.push(ReplayInstruction::PopClip)
    }
}

#[derive(Debug)]
pub struct Type3<'a> {
    widths: Vec<f32>,
    encodings: HashMap<u8, String>,
    dict: Dict<'a>,
    // TODO: Don't automatically resolve glyph streams?
    char_procs: HashMap<String, Stream<'a>>,
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

        let char_procs = {
            let mut procs = HashMap::new();
            let dict = dict.get::<Dict>(CHAR_PROCS).unwrap_or_default();

            for name in dict.keys() {
                let prog = dict.get::<Stream>(name).unwrap();

                procs.insert(name.as_str().to_string(), prog.clone());
            }

            procs
        };

        let glyph_to_string = HashMap::new();
        let string_to_glyph = HashMap::new();

        Self {
            glyph_to_string: RefCell::new(glyph_to_string),
            string_to_glyph: RefCell::new(string_to_glyph),
            char_procs,
            glyph_counter: RefCell::new(1),
            widths,
            encodings,
            matrix,
            dict: dict.clone(),
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

    pub fn render_glyph(&self, glyph: GlyphId, context: &mut Context<'a>) -> Type3GlyphDescription {
        let mut t3 =
            Type3GlyphDescription::new(self.matrix * Affine::scale_non_uniform(1000.0, 1000.0));

        let borrowed = self.glyph_to_string.borrow();
        let name = borrowed.get(&glyph).unwrap();
        let program = self.char_procs.get(name).unwrap();
        let decoded = program.decoded().unwrap();
        let resources = self.dict.get(RESOURCES).unwrap_or_default();

        let iter = TypedIter::new(UntypedIter::new(decoded.as_ref()));

        context.save_state();
        context.get_mut().affine = Affine::IDENTITY;
        interpret(iter, &resources, context, &mut t3);
        context.restore_state();

        t3
    }
}
