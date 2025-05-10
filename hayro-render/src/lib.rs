use crate::paint::Image;
use crate::pixmap::Pixmap;
use crate::render::RenderContext;
use hayro_interpret::color::Color;
use hayro_interpret::context::Context;
use hayro_interpret::device::{ClipPath, Device, Mask};
use hayro_interpret::{FillProps, StrokeProps, interpret};
use hayro_syntax::document::page::{Page, Rotation};
use hayro_syntax::pdf::Pdf;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use kurbo::{Affine, BezPath, Rect};
use peniko::color::palette::css::WHITE;
use peniko::color::{AlphaColor, Srgb};
use peniko::{Fill, ImageQuality};
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
mod render;
mod strip;
mod tile;
mod util;

#[derive(Debug, Copy, Clone)]
pub enum RenderMode {
    OptimizeQuality,
    OptimizeSpeed,
}

struct Renderer(RenderContext);

impl Renderer {
    fn draw_image(&mut self, image_data: Vec<u8>, width: u32, height: u32, is_stencil: bool) {
        let premul = image_data
            .chunks_exact(4)
            .map(|d| {
                AlphaColor::<Srgb>::from_rgba8(d[0], d[1], d[2], d[3])
                    .premultiply()
                    .to_rgba8()
            })
            .collect();
        let pixmap = Pixmap::from_parts(premul, width as u16, height as u16);
        pixmap.clone().save_png("pix.png");

        let image = Image {
            pixmap: Arc::new(pixmap),
            x_extend: Default::default(),
            y_extend: Default::default(),
            quality: ImageQuality::Low,
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

    fn stroke_path(&mut self, path: &BezPath, stroke_props: &StrokeProps) {
        let stroke = kurbo::Stroke {
            width: stroke_props.line_width as f64,
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

    fn apply_mask(&mut self, mask: &Mask) {
        todo!()
    }

    fn draw_rgba_image(&mut self, image_data: Vec<u8>, width: u32, height: u32) {
        self.draw_image(image_data, width, height, false);
    }

    fn draw_stencil_image(&mut self, image_data: Vec<u8>, width: u32, height: u32) {
        println!("{:?}", image_data.len());
        self.draw_image(image_data, width, height, true);
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
    let mut state = Context::new(initial_transform);
    let mut device = Renderer(RenderContext::new(pix_width, pix_height));

    device.0.set_paint(WHITE);
    device
        .0
        .fill_rect(&Rect::new(0.0, 0.0, pix_width as f64, pix_height as f64));

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
