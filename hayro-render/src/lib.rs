use crate::encode::x_y_advances;
use crate::paint::Image;
use crate::pixmap::Pixmap;
use crate::render::RenderContext;
use hayro_interpret::color::Color;
use hayro_interpret::context::Context;
use hayro_interpret::device::{ClipPath, Device};
use hayro_interpret::pattern::ShadingPattern;
use hayro_interpret::{FillProps, StrokeProps, interpret};
use hayro_syntax::document::page::{Page, Rotation};
use hayro_syntax::pdf::Pdf;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageBuffer, ImageEncoder};
use kurbo::{Affine, BezPath, Point, Rect};
use peniko::Fill;
use peniko::color::palette::css::WHITE;
use peniko::color::{AlphaColor, Srgb};
use std::io::Cursor;
use std::ops::RangeInclusive;
use std::sync::Arc;

mod coarse;
mod encode;
mod fine;
mod flatten;
mod mask;
mod paint;
mod pixmap;
pub mod render;
mod strip;
mod tile;

struct Renderer(RenderContext);

impl Renderer {
    fn draw_image(
        &mut self,
        image_data: Vec<u8>,
        mut width: u32,
        mut height: u32,
        is_stencil: bool,
        interpolate: bool,
    ) {
        let mut cur_transform = self.0.transform;

        let (x_scale, y_scale) = {
            let (x, y) = x_y_advances(&cur_transform);
            (x.length() as f32, y.length() as f32)
        };

        let image_data = if x_scale >= 1.0 && y_scale >= 1.0 {
            image_data
        } else {
            // Do subsampling to prevent aliasing artifacts.
            let new_width = (width as f32 * x_scale).ceil().max(1.0) as u32;
            let new_height = (height as f32 * y_scale).ceil().max(1.0) as u32;

            let image = DynamicImage::ImageRgba8(
                ImageBuffer::from_raw(width, height, image_data.clone()).unwrap(),
            );
            let resized = image.resize_exact(new_width, new_height, FilterType::CatmullRom);

            let new_width = resized.width();
            let new_height = resized.height();
            let t_scale_x = width as f32 / new_width as f32;
            let t_scale_y = height as f32 / new_height as f32;

            cur_transform =
                cur_transform * Affine::scale_non_uniform(t_scale_x as f64, t_scale_y as f64);
            self.0.set_transform(cur_transform);

            width = new_width;
            height = new_height;

            resized.to_rgba8().into_raw()
        };

        let premul = image_data
            .chunks_exact(4)
            .map(|d| {
                AlphaColor::<Srgb>::from_rgba8(d[0], d[1], d[2], d[3])
                    .premultiply()
                    .to_rgba8()
            })
            .collect();
        let pixmap = Pixmap::from_parts(premul, width as u16, height as u16);

        let image = Image {
            pixmap: Arc::new(pixmap),
            x_extend: Default::default(),
            x_step: width as f32,
            y_step: height as f32,
            y_extend: Default::default(),
            interpolate,
            is_stencil,
        };

        self.0.set_paint(image);
        self.0
            .fill_rect(&Rect::new(0.0, 0.0, width as f64, height as f64));
    }
}

impl Device for Renderer {
    fn set_transform(&mut self, affine: Affine) {
        self.0.set_transform(affine);
    }

    fn set_paint(&mut self, color: Color) {
        let res = color.to_rgba();
        self.0.set_paint(res);
    }

    fn set_shading_paint(&mut self, color: ShadingPattern) {
        self.0.set_paint(color);
    }

    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps) {
        // Best-effort attempt to ensure a line width of at least 1.
        let min_factor = min_factor(&self.0.transform);
        let mut line_width = stroke_props.line_width.max(0.01);
        let transformed_width = line_width * min_factor;

        if transformed_width < 1.0 {
            line_width = line_width / transformed_width;
        }

        let stroke = kurbo::Stroke {
            width: line_width as f64,
            join: stroke_props.line_join,
            miter_limit: stroke_props.miter_limit as f64,
            start_cap: stroke_props.line_cap,
            end_cap: stroke_props.line_cap,
            dash_pattern: stroke_props.dash_array.iter().map(|n| *n as f64).collect(),
            dash_offset: stroke_props.dash_offset as f64,
        };

        self.0.set_stroke(stroke);
        self.0.stroke_path(path);
    }

    fn fill_path(&mut self, path: &BezPath, fill_props: &FillProps) {
        self.0.set_fill_rule(fill_props.fill_rule);
        self.0.fill_path(path);
    }

    fn push_layer(&mut self, clip: Option<&ClipPath>, opacity: f32) {
        self.0
            .set_fill_rule(clip.map(|c| c.fill).unwrap_or(Fill::NonZero));
        self.0
            .push_layer(clip.map(|c| &c.path), None, Some(opacity), None)
    }

    fn draw_rgba_image(
        &mut self,
        image_data: Vec<u8>,
        width: u32,
        height: u32,
        is_stencil: bool,
        interpolate: bool,
    ) {
        self.draw_image(image_data, width, height, is_stencil, interpolate);
    }

    fn pop(&mut self) {
        self.0.pop_layer();
    }
}

pub fn render(page: &Page, scale: f32) -> Pixmap {
    let crop_box = page.crop_box();

    let (unscaled_width, unscaled_height) = (crop_box.width(), crop_box.height());
    let (mut pix_width, mut pix_height) = (unscaled_width, unscaled_height);

    let rotation_transform = Affine::scale(scale as f64)
        * match page.rotation() {
            Rotation::None => Affine::IDENTITY,
            Rotation::Horizontal => {
                let t = Affine::rotate(90.0f64.to_radians())
                    * Affine::translate((0.0, -unscaled_height));
                std::mem::swap(&mut pix_width, &mut pix_height);

                t
            }
            Rotation::Flipped => {
                Affine::scale(-1.0) * Affine::translate((-unscaled_width, -unscaled_height))
            }
            Rotation::FlippedHorizontal => {
                let t = Affine::translate((0.0, unscaled_width))
                    * Affine::rotate(-90.0f64.to_radians());
                std::mem::swap(&mut pix_width, &mut pix_height);

                t
            }
        };

    let initial_transform = rotation_transform
        * Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, unscaled_height])
        * Affine::translate((-crop_box.x0, -crop_box.y0));

    let (scaled_width, scaled_height) = (
        (pix_width as f32 * scale) as f64,
        (pix_height as f32 * scale) as f64,
    );
    let (pix_width, pix_height) = (scaled_width.floor() as u16, scaled_height.floor() as u16);
    let mut state = Context::new(
        initial_transform,
        kurbo::Rect::new(0.0, 0.0, pix_width as f64, pix_height as f64),
        page.xref(),
    );
    let mut device = Renderer(RenderContext::new(pix_width, pix_height));

    device.0.set_paint(WHITE);
    device
        .0
        .fill_rect(&Rect::new(0.0, 0.0, pix_width as f64, pix_height as f64));

    device.set_transform(initial_transform);

    device.push_layer(None, 1.0);
    interpret(
        page.typed_operations(),
        &page.resources(),
        &mut state,
        &mut device,
    );
    device.pop();

    let mut pixmap = Pixmap::new(pix_width, pix_height);
    device.0.render_to_pixmap(&mut pixmap);
    pixmap
}

pub fn render_png(pdf: &Pdf, scale: f32, range: Option<RangeInclusive<usize>>) -> Vec<Vec<u8>> {
    pdf.pages()
        .unwrap()
        .pages
        .iter()
        .enumerate()
        .flat_map(|(idx, page)| {
            if range.clone().is_some_and(|range| !range.contains(&idx)) {
                return None;
            }

            let pixmap = render(page, scale);

            let mut png_data = Vec::new();
            let cursor = Cursor::new(&mut png_data);
            let encoder = PngEncoder::new(cursor);
            encoder
                .write_image(
                    pixmap.data_as_u8_slice(),
                    pixmap.width() as u32,
                    pixmap.height() as u32,
                    ExtendedColorType::Rgba8,
                )
                .expect("Failed to encode image");

            Some(png_data)
        })
        .collect()
}

pub(crate) fn min_factor(transform: &Affine) -> f32 {
    let scale_skew_transform = {
        let c = transform.as_coeffs();
        Affine::new([c[0], c[1], c[2], c[3], 0.0, 0.0])
    };

    let x_advance = scale_skew_transform * Point::new(1.0, 0.0);
    let y_advance = scale_skew_transform * Point::new(0.0, 1.0);

    x_advance
        .to_vec2()
        .length()
        .min(y_advance.to_vec2().length()) as f32
}
