// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::encode::image::EncodedImage;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS, from_rgba8, Sampler};
use kurbo::{Point, Vec2};

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
        let mut pos = self.cur_pos;

        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            let sample =
                sample_with_interpolation(self.image, pos, self.image.interpolate);
            pixel.copy_from_slice(&sample);
            pos += self.image.y_advance;
        }
    }
}

impl Painter for ImageFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}

/// Sample a point with optional interpolation, applying the given extend function
pub(crate) fn sample_with_interpolation(
    sampleable: &impl Sampler,
    point: Point,
    interpolate: bool,
) -> [f32; 4]
{
    if !interpolate {
        sampleable.sample(point)
    } else {
        fn fract(val: f32) -> f32 {
            val - val.floor()
        }

        let x_fract = fract(point.x as f32 + 0.5);
        let y_fract = fract(point.y as f32 + 0.5);

        let mut interpolated_color = [0.0_f32; 4];

        let cx = [1.0 - x_fract, x_fract];
        let cy = [1.0 - y_fract, y_fract];

        for (x_idx, x) in [-0.5, 0.5].into_iter().enumerate() {
            for (y_idx, y) in [-0.5, 0.5].into_iter().enumerate() {
                let color_sample = sampleable.sample(point + Vec2::new(x, y));
                let w = cx[x_idx] * cy[y_idx];

                for (component, component_sample) in interpolated_color.iter_mut().zip(color_sample)
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

        interpolated_color
    }
}

impl Sampler for EncodedImage {
    fn sample(&self, pos: Point) -> [f32; 4] {
        let mut x = pos.x as u16;
        let mut y = pos.y as u16;
        
        x = x.clamp(0, self.pixmap.width() - 1);
        y = y.clamp(0, self.pixmap.height() - 1);
        
        from_rgba8(&self.pixmap.sample(x, y).to_u8_array())
    }
}
