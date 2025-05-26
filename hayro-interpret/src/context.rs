use crate::color::ColorSpace;
use crate::convert::convert_transform;
use crate::font::Font;
use crate::state::{State, TextState};
use crate::{FillProps, StrokeProps};
use hayro_syntax::content::ops::Transform;
use hayro_syntax::document::page::Resources;
use hayro_syntax::file::xref::XRef;
use hayro_syntax::object::Object;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::name::Name;
use hayro_syntax::object::r#ref::ObjRef;
use kurbo::{Affine, BezPath, Cap, Join, Point};
use peniko::Fill;
use smallvec::smallvec;
use std::collections::HashMap;

pub struct Context<'a> {
    states: Vec<State<'a>>,
    path: BezPath,
    sub_path_start: Point,
    last_point: Point,
    clip: Option<Fill>,
    font_cache: HashMap<ObjRef, Font<'a>>,
    root_transforms: Vec<Affine>,
    bbox: Vec<kurbo::Rect>,
    color_space_cache: HashMap<ObjRef, ColorSpace>,
    xref: XRef<'a>,
}

impl<'a> Context<'a> {
    pub fn new(initial_transform: Affine, bbox: kurbo::Rect, xref: XRef<'a>) -> Self {
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
                stroke_pattern: None,
                fill_pattern: None,
            }],
            xref,
            root_transforms: vec![initial_transform],
            last_point: Point::default(),
            sub_path_start: Point::default(),
            clip: None,
            bbox: vec![bbox],
            path: BezPath::new(),
            font_cache: HashMap::new(),
            color_space_cache: HashMap::new(),
        }
    }

    pub(crate) fn save_state(&mut self) {
        let cur = self.states.last().unwrap().clone();
        self.states.push(cur);
    }

    pub(crate) fn bbox(&self) -> kurbo::Rect {
        *self.bbox.last().unwrap()
    }

    pub(crate) fn push_root_transform(&mut self) {
        self.root_transforms.push(self.get().affine);
    }

    pub(crate) fn pop_root_transform(&mut self) {
        self.root_transforms.pop();
    }

    pub(crate) fn root_transform(&self) -> &Affine {
        self.root_transforms.last().unwrap()
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

    pub(crate) fn get(&self) -> &State<'a> {
        self.states.last().unwrap()
    }

    pub(crate) fn get_mut(&mut self) -> &mut State<'a> {
        self.states.last_mut().unwrap()
    }

    pub(crate) fn pre_concat_transform(&mut self, transform: Transform) {
        self.pre_concat_affine(convert_transform(transform))
    }

    pub(crate) fn pre_concat_affine(&mut self, transform: Affine) {
        self.get_mut().affine *= transform;
    }

    pub(crate) fn get_font(&mut self, resources: &Resources<'a>, name: Name) -> Font<'a> {
        resources
            .get_font(
                &name,
                |ref_| {
                    Some(
                        self.font_cache
                            .entry(ref_)
                            .or_insert_with(|| {
                                resources
                                    .resolve_ref::<Dict>(ref_)
                                    .map(|o| Font::new(&o).unwrap())
                                    .unwrap()
                            })
                            .clone(),
                    )
                },
                |c| Font::new(&c),
            )
            .unwrap()
    }

    pub(crate) fn get_color_space(&mut self, resources: &Resources, name: Name) -> ColorSpace {
        resources
            .get_color_space(
                &name,
                |ref_| {
                    Some(
                        self.color_space_cache
                            .entry(ref_)
                            .or_insert_with(|| {
                                resources
                                    .resolve_ref::<Object>(ref_)
                                    .map(|o| ColorSpace::new(o))
                                    .unwrap()
                            })
                            .clone(),
                    )
                },
                |c| Some(ColorSpace::new(c)),
            )
            .unwrap()
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

    pub fn xref(&self) -> &XRef<'a> {
        &self.xref
    }
}
