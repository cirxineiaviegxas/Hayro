mod convert;

use hayro_syntax::content::ops::{LineCap, Transform, TypedOperation};
use hayro_syntax::pdf::Pdf;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::io::Cursor;
use vello_api::color::AlphaColor;
use vello_api::color::palette::css::WHITE;
use vello_api::kurbo::{Affine, BezPath, Cap, Join, Point, Rect, Stroke};
use vello_cpu::{Pixmap, RenderContext};
use crate::convert::{convert_line_cap, convert_line_join, convert_transform};

#[derive(Clone)]
enum Cs {
    DeviceRgb,
    DeviceGray
}

#[derive(Clone)]
struct State {
    pub line_width: f32,
    pub line_cap: Cap,
    pub line_join: Join,
    pub miter_limit: f32,
    pub affine: Affine,
    pub stroke_cs: Cs,
    pub stroke_color: Vec<f32>,
    pub fill_color: Vec<f32>,
    pub fill_cs: Cs,
}

struct GraphicsState {
    states: Vec<State>,
    path: BezPath,
}

impl GraphicsState {
    pub fn new(height: f32) -> Self {
        let line_width = 1.0;
        let line_cap = Cap::Butt;
        let line_join = Join::Miter;
        let miter_limit = 10.0;
        let affine = Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, height as f64]);

        Self {
            states: vec![State {
                line_width,
                line_cap,
                line_join,
                miter_limit,
                affine,
                stroke_cs: Cs::DeviceRgb,
                stroke_color: vec![0.0, 0.0, 0.0],
                fill_color: vec![0.0, 0.0, 0.0],
                fill_cs: Cs::DeviceRgb,
            }],
            path: BezPath::new(),
        }
    }

    pub fn save_state(&mut self) {
        let cur = self.states.last().unwrap().clone();
        self.states.push(cur);
    }

    pub fn set_stroke_color(&mut self, col: Vec<f32>) {
        self.cur_mut().stroke_color = col;
    }

    pub fn restore_state(&mut self) {
        self.states.pop();
    }

    fn cur(&self) -> &State {
        self.states.last().unwrap()
    }

    pub fn cur_mut(&mut self) -> &mut State {
        self.states.last_mut().unwrap()
    }

    pub fn ctm(&mut self, transform: Transform) {
        self.cur_mut().affine *= convert_transform(transform);
    }
}

pub fn render(pdf: &Pdf) -> Pixmap {
    let pages = &pdf.pages().unwrap().pages[0];
    let (width, height) = (pages.media_box()[2], pages.media_box()[3]);
    let (pix_width, pix_height) = (width.ceil() as u16, height.ceil() as u16);
    let mut state = GraphicsState::new(height);
    let mut render_ctx = RenderContext::new(pix_width, pix_height);
    
    render_ctx.set_paint(WHITE);
    render_ctx.fill_rect(&Rect::new(0.0, 0.0, pix_width as f64, pix_height as f64));

    for op in pages.typed_operations() {
        match op {
            TypedOperation::SaveState(_) => state.save_state(),
            TypedOperation::StrokeColorDeviceRgb(s) => {
                state.cur_mut().stroke_cs = Cs::DeviceRgb;
                state.cur_mut().stroke_color = vec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32()];
            }
            TypedOperation::LineWidth(w) => {
                state.cur_mut().line_width = w.0.as_f32();
            }
            TypedOperation::LineCap(c) => {
                state.cur_mut().line_cap = convert_line_cap(c);
            }
            TypedOperation::LineJoin(j) => {
                state.cur_mut().line_join = convert_line_join(j);
            }
            TypedOperation::MiterLimit(l) => {
                state.cur_mut().miter_limit = l.0.as_f32();
            }
            TypedOperation::Transform(t) => {
                state.ctm(t);
            }
            TypedOperation::MoveTo(m) => {
                state.path.move_to(Point::new(m.0.as_f64(), m.1.as_f64()));
            }
            TypedOperation::LineTo(m) => {
                state.path.line_to(Point::new(m.0.as_f64(), m.1.as_f64()));
            }
            TypedOperation::ClosePath(_) => {
                state.path.close_path();
            }
            TypedOperation::StrokePath(_) => {
                let aff = state.cur().affine;
                let path = std::mem::replace(&mut state.path, BezPath::new());
                let transformed_path = aff * path;
                
                let stroke = Stroke {
                    width: state.cur().line_width as f64,
                    join: state.cur().line_join,
                    miter_limit: state.cur().miter_limit as f64,
                    start_cap: state.cur().line_cap,
                    end_cap: state.cur().line_cap,
                    dash_pattern: Default::default(),
                    dash_offset: 0.0,
                };
                
                render_ctx.set_stroke(stroke);
                let color = {
                    let c = &state.cur().stroke_color;
                    
                    match state.cur().stroke_cs {
                        Cs::DeviceRgb => AlphaColor::new([c[0], c[1], c[2], 1.0]),
                        Cs::DeviceGray => AlphaColor::new([c[0], c[0], c[0], 1.0]),
                    }
                };
                
                println!("Stroking {:?}", transformed_path);
                
                render_ctx.set_paint(color);
                render_ctx.stroke_path(&transformed_path);
            }
            TypedOperation::RestoreState(_) => state.restore_state(),
            _ => {
                println!("{:?}", op);
            }
        }
    }

    let mut pixmap = Pixmap::new(pix_width, pix_height);
    render_ctx.render_to_pixmap(&mut pixmap);
    pixmap
}

pub fn render_png(pdf: &Pdf) -> Vec<u8> {
    let pixmap = render(pdf);

    let mut png_data = Vec::new();
    let cursor = Cursor::new(&mut png_data);
    let encoder = PngEncoder::new(cursor);
    encoder
        .write_image(
            pixmap.data(),
            pixmap.width() as u32,
            pixmap.height() as u32,
            ExtendedColorType::Rgba8,
        )
        .expect("Failed to encode image");

    png_data
}
