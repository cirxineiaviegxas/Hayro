use crate::color::{ColorComponents, ColorSpace};
use crate::convert::convert_transform;
use crate::font::{Font, TextRenderingMode};
use crate::{FillProps, StrokeProps};
use hayro_syntax::content::ops::Transform;
use kurbo::{Affine, BezPath, Cap, Join, Point};
use peniko::Fill;
use smallvec::{SmallVec, smallvec};

#[derive(Clone)]
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

#[derive(Clone)]
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

pub struct GraphicsState {
    states: Vec<State>,
    path: BezPath,
    sub_path_start: Point,
    last_point: Point,
    clip: Option<Fill>,
}

impl GraphicsState {
    pub fn new(initial_transform: Affine) -> Self {
        let line_width = 1.0;
        let line_cap = Cap::Butt;
        let line_join = Join::Miter;
        let miter_limit = 10.0;

        Self {
            states: vec![State {
                line_width,
                line_cap,
                line_join,
                miter_limit,
                dash_array: smallvec![],
                dash_offset: 0.0,
                affine: initial_transform,
                fill_alpha: 1.0,
                stroke_cs: ColorSpace::DeviceGray,
                stroke_color: smallvec![0.0,],
                fill_cs: ColorSpace::DeviceGray,
                fill_color: smallvec![0.0],
                stroke_alpha: 1.0,
                fill: Fill::NonZero,
                n_clips: 0,
                text_state: TextState::default(),
            }],
            last_point: Point::default(),
            sub_path_start: Point::default(),
            clip: None,
            path: BezPath::new(),
        }
    }

    pub(crate) fn save_state(&mut self) {
        let cur = self.states.last().unwrap().clone();
        self.states.push(cur);
    }

    pub(crate) fn restore_state(&mut self) {
        self.states.pop();
    }

    pub(crate) fn path(&self) -> &BezPath {
        &self.path
    }

    pub(crate) fn path_mut(&mut self) -> &mut BezPath {
        &mut self.path
    }

    pub(crate) fn sub_path_start(&self) -> &Point {
        &self.sub_path_start
    }

    pub(crate) fn sub_path_start_mut(&mut self) -> &mut Point {
        &mut self.sub_path_start
    }

    pub(crate) fn last_point(&self) -> &Point {
        &self.last_point
    }

    pub(crate) fn last_point_mut(&mut self) -> &mut Point {
        &mut self.last_point
    }

    pub(crate) fn clip(&self) -> &Option<Fill> {
        &self.clip
    }

    pub(crate) fn clip_mut(&mut self) -> &mut Option<Fill> {
        &mut self.clip
    }

    pub(crate) fn get(&self) -> &State {
        self.states.last().unwrap()
    }

    pub(crate) fn get_mut(&mut self) -> &mut State {
        self.states.last_mut().unwrap()
    }

    pub(crate) fn pre_concat_transform(&mut self, transform: Transform) {
        self.get_mut().affine *= convert_transform(transform);
    }

    pub(crate) fn stroke_props(&self) -> StrokeProps {
        let state = self.get();

        StrokeProps {
            line_width: state.line_width,
            line_cap: state.line_cap,
            line_join: state.line_join,
            miter_limit: state.miter_limit,
            dash_array: state.dash_array.clone(),
            dash_offset: state.dash_offset,
        }
    }

    pub(crate) fn fill_props(&self) -> FillProps {
        let state = self.get();

        FillProps {
            fill_rule: state.fill,
        }
    }
}
