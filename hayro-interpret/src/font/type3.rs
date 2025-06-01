use crate::clip_path::ClipPath;
use crate::context::Context;
use crate::device::{Device, ReplayInstruction};
use crate::font::UNITS_PER_EM;
use crate::font::true_type::{read_encoding, read_widths};
use crate::font::type1::GlyphSimulator;
use crate::image::{RgbaImage, StencilImage};
use crate::paint::Paint;
use crate::{FillProps, StrokeProps, interpret};
use hayro_syntax::content::{TypedIter, UntypedIter};
use hayro_syntax::document::page::Resources;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{CHAR_PROCS, FONT_MATRIX, RESOURCES};
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, BezPath};
use skrifa::GlyphId;
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

    fn set_paint_transform(&mut self, affine: Affine) {
        self.0.push(ReplayInstruction::SetPaintTransform { affine });
    }

    fn set_paint(&mut self, _: Paint) {}

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

    fn push_layer(&mut self, clip_path: Option<&ClipPath>, opacity: f32) {
        self.0.push(ReplayInstruction::PushLayer {
            clip: clip_path.cloned(),
            opacity,
        });
    }

    fn draw_rgba_image(&mut self, image: RgbaImage) {
        self.0.push(ReplayInstruction::DrawImage { image })
    }

    fn draw_stencil_image(&mut self, stencil: StencilImage) {
        self.0.push(ReplayInstruction::DrawStencil {
            stencil_image: stencil,
        })
    }

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
    glyph_simulator: GlyphSimulator,
    pub(crate) matrix: Affine,
}

impl<'a> Type3<'a> {
    pub fn new(dict: &Dict<'a>) -> Self {
        let (_, encodings) = read_encoding(dict);
        let widths = read_widths(dict, dict);

        let matrix = Affine::new(
            dict.get::<[f64; 6]>(FONT_MATRIX)
                .unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]),
        );

        let char_procs = {
            let mut procs = HashMap::new();
            let dict = dict.get::<Dict>(CHAR_PROCS).unwrap_or_default();

            for name in dict.keys() {
                let prog = dict.get::<Stream>(&name).unwrap();

                procs.insert(name.as_str().to_string(), prog.clone());
            }

            procs
        };

        Self {
            glyph_simulator: GlyphSimulator::new(),
            char_procs,
            widths,
            encodings,
            matrix,
            dict: dict.clone(),
        }
    }

    pub fn map_code(&self, code: u8) -> GlyphId {
        self.encodings
            .get(&code)
            .map(|g| self.glyph_simulator.string_to_glyph(g))
            .unwrap_or(GlyphId::NOTDEF)
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        (*self.widths.get(code as usize).unwrap() * self.matrix.as_coeffs()[0] as f32)
            * UNITS_PER_EM
    }

    pub fn render_glyph(&self, glyph: GlyphId, context: &mut Context<'a>) -> Type3GlyphDescription {
        let mut t3 = Type3GlyphDescription::new(self.matrix * Affine::scale(UNITS_PER_EM as f64));

        let name = self.glyph_simulator.glyph_to_string(glyph).unwrap();
        let program = self.char_procs.get(&name).unwrap();
        let decoded = program.decoded().unwrap();
        // TODO: Can resources be inherited?
        let resources = Resources::new(
            self.dict.get(RESOURCES).unwrap_or_default(),
            None,
            context.xref(),
        );

        let iter = TypedIter::new(UntypedIter::new(decoded.as_ref()));

        context.save_state();
        context.get_mut().ctm = Affine::IDENTITY;
        interpret(iter, &resources, context, &mut t3);
        context.restore_state();

        t3
    }
}
