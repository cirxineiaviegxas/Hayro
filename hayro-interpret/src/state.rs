use crate::color::{ColorComponents, ColorSpace};
use crate::convert::convert_transform;
use crate::font::{Font, TextRenderingMode};
use crate::{FillProps, StrokeProps};
use hayro_syntax::content::ops::Transform;
use kurbo::{Affine, BezPath, Cap, Join, Point};
use peniko::Fill;
use smallvec::{SmallVec, smallvec};

#[derive(Clone, Debug)]
pub(crate) struct TextState {
    pub(crate) char_space: f32,
    pub(crate) word_space: f32,
    pub(crate) horizontal_scaling: f32,
    pub(crate) leading: f32,
    pub(crate) font: Option<(Font, f32)>,
    pub(crate) render_mode: TextRenderingMode,
    pub(crate) text_matrix: Affine,
    pub(crate) text_line_matrix: Affine,
    pub(crate) rise: f32,
}

impl TextState {
    fn temp_transform(&self) -> Affine {
        Affine::new([
            self.font_size() as f64 * self.horizontal_scaling() as f64,
            0.0,
            0.0,
            self.font_size() as f64,
            0.0,
            self.rise as f64,
        ])
    }

    fn horizontal_scaling(&self) -> f32 {
        self.horizontal_scaling / 100.0
    }

    pub(crate) fn font_size(&self) -> f32 {
        self.font.as_ref().map(|f| f.1).unwrap_or(1.0)
    }

    pub(crate) fn font(&self) -> Font {
        self.font.as_ref().map(|f| f.0.clone()).unwrap()
    }

    pub(crate) fn step(&mut self, glyph_width: f32, positional_adjustment: f32) {
        // TODO: Vertical writing
        let tx = ((glyph_width - positional_adjustment) * self.font_size()
            + self.char_space
            + self.word_space)
            * self.horizontal_scaling();
        self.text_matrix = self.text_matrix * Affine::new([1.0, 0.0, 0.0, 1.0, tx as f64, 0.0]);
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            char_space: 0.0,
            word_space: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            font: None,
            render_mode: Default::default(),
            text_matrix: Affine::IDENTITY,
            text_line_matrix: Affine::IDENTITY,
            rise: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct State {
    pub(crate) line_width: f32,
    pub(crate) line_cap: Cap,
    pub(crate) line_join: Join,
    pub(crate) miter_limit: f32,
    pub(crate) dash_array: SmallVec<[f32; 4]>,
    pub(crate) dash_offset: f32,
    pub(crate) affine: Affine,
    pub(crate) stroke_color: ColorComponents,
    pub(crate) stroke_cs: ColorSpace,
    pub(crate) stroke_alpha: f32,
    pub(crate) fill_color: ColorComponents,
    pub(crate) fill_cs: ColorSpace,
    pub(crate) fill_alpha: f32,
    pub(crate) text_state: TextState,
    // Strictly speaking not part of the graphics state, but we keep it there for
    // consistency.
    pub(crate) fill: Fill,
    pub(crate) n_clips: u32,
}

impl State {
    pub(crate) fn text_transform(&self) -> Affine {
        self.affine * self.text_state.text_matrix * self.text_state.temp_transform()
    }
}
