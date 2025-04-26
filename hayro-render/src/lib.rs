mod convert;

use crate::convert::{convert_line_cap, convert_line_join, convert_transform};
use hayro_syntax::content::ops::{LineCap, Transform, TypedOperation};
use hayro_syntax::pdf::Pdf;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::io::Cursor;
use vello_api::color::AlphaColor;
use vello_api::color::palette::css::WHITE;
use vello_api::kurbo::{Affine, BezPath, Cap, Join, Point, Rect, Shape, Stroke};
use vello_api::peniko::Fill;
use vello_cpu::{Pixmap, RenderContext};

#[derive(Clone)]
enum Cs {
    DeviceRgb,
    DeviceGray,
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
                affine: initial_transform,
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

pub fn render(pdf: &Pdf, scale: f32) -> Pixmap {
    let pages = &pdf.pages().unwrap().pages[0];
    let (unscaled_width, unscaled_height) = (pages.media_box()[2], pages.media_box()[3]);
    let initial_transform = Affine::scale(scale as f64)
        * Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, unscaled_height as f64]);
    let (scaled_width, scaled_height) = (
        (unscaled_width * scale) as f64,
        (unscaled_height * scale) as f64,
    );
    let (pix_width, pix_height) = (scaled_width.ceil() as u16, scaled_height.ceil() as u16);
    let mut state = GraphicsState::new(initial_transform);
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
            TypedOperation::RectPath(r) => {
                let rect = Rect::new(
                    r.0.as_f64(),
                    r.1.as_f64(),
                    r.0.as_f64() + r.2.as_f64(),
                    r.1.as_f64() + r.3.as_f64(),
                )
                .to_path(0.1);
                state.path.extend(rect);
            }
            TypedOperation::MoveTo(m) => {
                state.path.move_to(Point::new(m.0.as_f64(), m.1.as_f64()));
            }
            TypedOperation::FillPathEvenOdd(_) => {
                render_ctx.set_fill_rule(Fill::EvenOdd);
                let color = {
                    let c = &state.cur().fill_color;

                    match state.cur().fill_cs {
                        Cs::DeviceRgb => AlphaColor::new([c[0], c[1], c[2], 1.0]),
                        Cs::DeviceGray => AlphaColor::new([c[0], c[0], c[0], 1.0]),
                    }
                };

                println!("{:?}", state.path);

                render_ctx.set_paint(color);
                render_ctx.set_transform(state.cur().affine);
                render_ctx.fill_path(&state.path);

                state.path.truncate(0);
            }
            TypedOperation::NonStrokeColorDeviceGray(d) => {
                state.cur_mut().fill_cs = Cs::DeviceGray;
                state.cur_mut().fill_color = vec![d.0.as_f32()];
            }
            TypedOperation::LineTo(m) => {
                state.path.line_to(Point::new(m.0.as_f64(), m.1.as_f64()));
            }
            TypedOperation::ClosePath(_) => {
                state.path.close_path();
            }
            TypedOperation::StrokePath(_) => {
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

                render_ctx.set_paint(color);
                render_ctx.set_transform(state.cur().affine);
                render_ctx.stroke_path(&state.path);

                state.path.truncate(0);
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
    let pixmap = render(pdf, 16.0);

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
