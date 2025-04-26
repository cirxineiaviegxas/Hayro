use hayro_interpret::device::Device;
use hayro_interpret::{FillProps, GraphicsState, StrokeProps, interpret};
use hayro_syntax::content::ops::{LineCap, Transform, TypedOperation};
use hayro_syntax::pdf::Pdf;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::io::Cursor;
use vello_api::color::palette::css::WHITE;
use vello_api::color::{AlphaColor, Srgb};
use vello_api::kurbo;
use vello_api::kurbo::{Affine, BezPath, Cap, Join, Point, Rect, Shape, Stroke};
use vello_api::peniko::Fill;
use vello_cpu::{Pixmap, RenderContext};

struct Renderer(RenderContext);

impl Device for Renderer {
    fn set_transform(&mut self, affine: Affine) {
        self.0.set_transform(affine);
    }

    fn set_paint(&mut self, color: AlphaColor<Srgb>) {
        self.0.set_paint(color);
    }

    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps) {
        let stroke = kurbo::Stroke {
            width: stroke_props.line_width as f64,
            join: stroke_props.line_join,
            miter_limit: stroke_props.miter_limit as f64,
            start_cap: stroke_props.line_cap,
            end_cap: stroke_props.line_cap,
            dash_pattern: Default::default(),
            dash_offset: 0.0,
        };

        self.0.set_stroke(stroke);
        self.0.stroke_path(path);
    }

    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps) {
        self.0.set_fill_rule(fill_props.fill_rule);
        self.0.fill_path(path);
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
    let mut device = Renderer(RenderContext::new(pix_width, pix_height));

    device.0.set_paint(WHITE);
    device
        .0
        .fill_rect(&Rect::new(0.0, 0.0, pix_width as f64, pix_height as f64));

    interpret(pages.typed_operations(), &mut state, &mut device);

    let mut pixmap = Pixmap::new(pix_width, pix_height);
    device.0.render_to_pixmap(&mut pixmap);
    pixmap
}

pub fn render_png(pdf: &Pdf) -> Vec<u8> {
    let pixmap = render(pdf, 1.0);

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
