use crate::color::{ColorComponents, ColorSpace};
use crate::font::{Font, TextRenderingMode, UNITS_PER_EM};
use kurbo::{Affine, BezPath, Cap, Join, Vec2};
use peniko::Fill;
use smallvec::SmallVec;

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
    pub(crate) clip_paths: BezPath,
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

    pub(crate) fn apply_adjustment(&mut self, adjustment: f32, horizontal: bool) {
        let horizontal_scaling = horizontal.then(|| self.horizontal_scaling()).unwrap_or(1.0);
        let scaled_adjustment = -adjustment / UNITS_PER_EM * self.font_size() * horizontal_scaling;
        let (tx, ty) = horizontal
            .then(|| (scaled_adjustment, 0.0))
            .unwrap_or((0.0, scaled_adjustment));

        self.text_matrix =
            self.text_matrix * Affine::new([1.0, 0.0, 0.0, 1.0, tx as f64, ty as f64]);
    }

    pub(crate) fn apply_glyph_width(
        &mut self,
        glyph_advance: Vec2,
        char_code: u16,
        code_len: usize,
        is_horizontal: bool,
    ) {
        let word_space = (char_code == 32 && code_len == 1)
            .then(|| self.word_space)
            .unwrap_or(0.0);

        let base_advance =
            |advance: f32| advance / UNITS_PER_EM * self.font_size() + self.char_space + word_space;

        let tx = is_horizontal
            .then(|| base_advance(glyph_advance.x as f32) * self.horizontal_scaling())
            .unwrap_or(0.0);
        let ty = (!is_horizontal)
            .then(|| base_advance(glyph_advance.y as f32))
            .unwrap_or(0.0);

        self.text_matrix =
            self.text_matrix * Affine::new([1.0, 0.0, 0.0, 1.0, tx as f64, ty as f64]);
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
            clip_paths: BezPath::default(),
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
        self.text_state.text_matrix * self.text_state.temp_transform()
    }
}
