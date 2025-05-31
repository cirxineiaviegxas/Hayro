// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::encode::image::EncodedImage;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS, from_rgba8};
use kurbo::{Point, Vec2};
use peniko::ImageQuality;

#[derive(Debug)]
pub(crate) struct ImageFiller<'a> {
    cur_pos: Point,
    image: &'a EncodedImage,
}

impl<'a> ImageFiller<'a> {
    pub(crate) fn new(image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        Self {
            cur_pos: image.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            image,
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
                point.x.floor() as f32,
                self.image.extends.0,
                self.image.x_step,
            ));
            point.y = f64::from(extend(
                point.y.floor() as f32,
                self.image.extends.1,
                self.image.y_step,
            ));

            point
        };

        let mut pos = self.cur_pos;

        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            match self.image.quality {
                ImageQuality::Low => {
                    let point = extend_point(pos);
                    let sample = from_rgba8(
                        &self
                            .image
                            .pixmap
                            .sample(point.x as u16, point.y as u16)
                            .to_u8_array(),
                    );
                    pixel.copy_from_slice(&sample);
                }
                ImageQuality::Medium => {
                    let fract = |orig_val: f64| {
                        let start = orig_val - 0.5;
                        let mut res = start.fract() as f32;

                        if res.is_sign_negative() {
                            res += 1.0;
                        }

                        res
                    };

                    let x_fract = fract(pos.x);
                    let y_fract = fract(pos.y);

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
                _ => unimplemented!(),
            };

            pos += self.image.y_advance;
        }
    }
}

fn extend(val: f32, extend: peniko::Extend, max: f32) -> f32 {
    match extend {
        peniko::Extend::Pad => val.clamp(0.0, max - 1.0),
        peniko::Extend::Repeat => val.rem_euclid(max),
        peniko::Extend::Reflect => {
            let period = 2.0 * max;

            let val_mod = val.rem_euclid(period);

            if val_mod < max {
                val_mod
            } else {
                (period - 1.0) - val_mod
            }
        }
    }
}

impl Painter for ImageFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}
