// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::encode::EncodedImage;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS, from_rgba8};
use kurbo::{Point, Vec2};
use peniko::ImageQuality;

#[derive(Debug)]
pub(crate) struct ImageFiller<'a> {
    /// The current position that should be processed.
    cur_pos: Point,
    /// The underlying image.
    image: &'a EncodedImage,
}

impl<'a> ImageFiller<'a> {
    pub(crate) fn new(image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        Self {
            // We want to sample values of the pixels at the center, so add an offset of 0.5.
            cur_pos: image.transform
                * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5),
            image,
        }
    }

    pub(super) fn run(mut self, target: &mut [f32]) {
        // Fallback path.
        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                self.run_complex_column(column);
                self.cur_pos += self.image.x_advance;
            });
    }

    fn run_complex_column(&mut self, col: &mut [f32]) {
        let extend_point = |mut point: Point| {
            // For the same reason as mentioned above, we always floor.
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
                // Nearest neighbor filtering.
                // Simply takes the nearest pixel to our current position.
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
                    // We have two versions of filtering: `Medium` (bilinear filtering) and
                    // `High` (bicubic filtering).

                    // In bilinear filtering, we sample the pixels of the rectangle that spans the
                    // locations (-0.5, -0.5) and (0.5, 0.5), and weight them by the fractional
                    // x/y position using simple linear interpolation in both dimensions.
                    // In bicubic filtering, we instead span a 4x4 grid around the
                    // center of the location we are sampling, and sample those points
                    // using a cubic filter to weight each location's contribution.

                    let fract = |orig_val: f64| {
                        // To give some intuition on why we need that shift, based on bilinear
                        // filtering: If we sample at the position (0.5, 0.5), we are at the center
                        // of the pixel and thus only want the color of the current pixel. Thus, we take
                        // 1.0 * 1.0 from the top left pixel (which still lies on our pixel)
                        // and 0.0 from all other corners (which lie at the start of other pixels).
                        //
                        // If we sample at the position (0.4, 0.4), we want 0.1 * 0.1 = 0.01 from
                        // the top-left pixel, 0.1 * 0.9 = 0.09 from the bottom-left and top-right,
                        // and finally 0.9 * 0.9 = 0.81 from the bottom right position (which still
                        // lies on our pixel, and thus has intuitively should have the highest
                        // contribution). Thus, we need to subtract 0.5 from the position to get
                        // the correct fractional contribution.
                        let start = orig_val - 0.5;
                        let mut res = start.fract() as f32;

                        // In case we are in the negative we need to mirror the result.
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

                    // Note that the sum of all cx*cy combinations also yields 1.0 again
                    // (modulo some floating point number impreciseness), ensuring the
                    // colors stay in range.

                    // We sample the corners rectangle that covers our current position.
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
                        // Due to the nature of the cubic filter, it can happen in certain situations
                        // that one of the color components ends up with a higher value than the
                        // alpha component, which isn't permissible because the color is
                        // premultiplied and would lead to overflows when doing source over
                        // compositing with u8-based values. Because of this, we need to clamp
                        // to the alpha value.
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
        // TODO: We need to make repeat and reflect more efficient and branch-less.
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
