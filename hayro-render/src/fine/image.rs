// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::encode::image::EncodedImage;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS, from_rgba8};
use kurbo::{Point, Vec2};

#[derive(Debug)]
pub(crate) struct ImageFiller<'a> {
    cur_pos: Point,
    image: &'a EncodedImage,
    height: f32,
    height_inv: f32,
    width: f32,
    width_inv: f32,
}

impl<'a> ImageFiller<'a> {
    pub(crate) fn new(image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        let height = image.pixmap.height() as f32;
        let width = image.pixmap.width() as f32;
        
        Self {
            cur_pos: image.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            image,
            width,
            width_inv: 1.0 / width,
            height,
            height_inv: 1.0 / height,
        }
    }

    pub(super) fn run(mut self, target: &mut [f32]) {
        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                self.run_complex_column(column);
                self.cur_pos += self.image.x_advance;
            });
    }

    fn run_complex_column(&mut self, col: &mut [f32]) {
        let extend_point = |mut point: Point| {
            point.x = f64::from(extend(
                point.x as f32,
                self.image.repeat,
                self.width,
                self.width_inv,
            ));
            point.y = f64::from(extend(
                point.y as f32,
                self.image.repeat,
                self.height,
                self.height_inv,
            ));

            point
        };

        let mut pos = self.cur_pos;

        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            if !self.image.interpolate {
                let point = extend_point(pos);
                let sample = from_rgba8(
                    &self
                        .image
                        .pixmap
                        .sample(point.x as u16, point.y as u16)
                        .to_u8_array(),
                );
                pixel.copy_from_slice(&sample);
            } else {
                fn fract(val: f32) -> f32 {
                    val - val.floor()
                }

                let x_fract = fract(pos.x as f32 + 0.5);
                let y_fract = fract(pos.y as f32 + 0.5);

                let mut interpolated_color = [0.0_f32; 4];

                let sample = |p: Point| {
                    let c = |val: u8| f32::from(val) / 255.0;
                    let s = self.image.pixmap.sample(p.x as u16, p.y as u16);

                    [c(s.r), c(s.g), c(s.b), c(s.a)]
                };

                let cx = [1.0 - x_fract, x_fract];
                let cy = [1.0 - y_fract, y_fract];

                for (x_idx, x) in [-0.5, 0.5].into_iter().enumerate() {
                    for (y_idx, y) in [-0.5, 0.5].into_iter().enumerate() {
                        let color_sample = sample(extend_point(pos + Vec2::new(x, y)));
                        let w = cx[x_idx] * cy[y_idx];

                        for (component, component_sample) in
                            interpolated_color.iter_mut().zip(color_sample)
                        {
                            *component += w * component_sample;
                        }
                    }
                }

                for i in 0..COLOR_COMPONENTS {
                    let f32_val = interpolated_color[i]
                        .clamp(0.0, 1.0)
                        .min(interpolated_color[3]);
                    interpolated_color[i] = f32_val;
                }

                pixel.copy_from_slice(&interpolated_color);
            }

            pos += self.image.y_advance;
        }
    }
}


#[inline(always)]
fn extend(val: f32, repeat: bool, max: f32, inv_max: f32) -> f32 {
    const BIAS: f32 = 0.01;

    if !repeat {
        val.clamp(0.0, max - BIAS)
    }   else {
        val - (val * inv_max).floor() * max
    }
}

impl Painter for ImageFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}
