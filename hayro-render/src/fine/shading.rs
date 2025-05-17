// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::encode::{EncodedAxialShading, EncodedFunctionShading};
use crate::fine::{COLOR_COMPONENTS, FineType, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use kurbo::{Point, Vec2};
use smallvec::smallvec;

#[derive(Debug)]
pub(crate) struct FunctionShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedFunctionShading,
}

impl<'a> FunctionShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedFunctionShading, start_x: u16, start_y: u16) -> Self {
        Self {
            // We want to sample values of the pixels at the center, so add an offset of 0.5.
            cur_pos: shading.inverse_transform
                * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5),
            shading,
        }
    }

    pub(super) fn run<F: FineType>(mut self, target: &mut [F]) {
        let bg_color = F::extract_color(&PremulColor::from_alpha_color(self.shading.background));

        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                self.run_complex_column(column, &bg_color);
                self.cur_pos += self.shading.x_advance;
            });
    }

    fn run_complex_column<F: FineType>(&mut self, col: &mut [F], bg_color: &[F; 4]) {
        let mut pos = self.cur_pos;

        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            if !self.shading.domain.contains(pos) {
                pixel.copy_from_slice(bg_color);
            } else {
                let out = self
                    .shading
                    .function
                    .eval(smallvec![pos.x as f32, pos.y as f32])
                    .unwrap();
                // TODO: CLamp out-of-range values.
                let color = self.shading.color_space.to_rgba(&out, 1.0);
                pixel.copy_from_slice(&F::extract_color(&PremulColor::from_alpha_color(color)));
            }
            pos += self.shading.y_advance;
        }
    }
}

impl Painter for FunctionShadingFiller<'_> {
    fn paint<F: FineType>(self, target: &mut [F]) {
        self.run(target);
    }
}

#[derive(Debug)]
pub(crate) struct AxialShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedAxialShading,
}

impl<'a> AxialShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedAxialShading, start_x: u16, start_y: u16) -> Self {
        Self {
            // We want to sample values of the pixels at the center, so add an offset of 0.5.
            cur_pos: shading.inverse_transform
                * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5),
            shading,
        }
    }

    pub(super) fn run<F: FineType>(mut self, target: &mut [F]) {
        let bg_color = F::extract_color(&PremulColor::from_alpha_color(self.shading.background));

        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                self.run_complex_column(column, &bg_color);
                self.cur_pos += self.shading.x_advance;
            });
    }

    fn run_complex_column<F: FineType>(&mut self, col: &mut [F], bg_color: &[F; 4]) {
        // TODO: If the
        // starting and ending coordinates are coincident (x0=x1 and y0=y1) nothing shall be
        // painted.
        
        let mut pos = self.cur_pos;
        let [x0, y0, x1, y1] = self.shading.coords;
        
        let (t0, t1) = (self.shading.domain[0], self.shading.domain[1]);

        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            let (x, y) = (pos.x as f32, pos.y as f32);
            let p1 = (x1 - x0) * (x - x0) + (y1 - y0) * (y - y0);
            let mut x = p1 / self.shading.denom;

            if x < 0.0 {
                if self.shading.extend[0] {
                    x = 0.0;
                } else {
                    pixel.copy_from_slice(bg_color);
                    continue;
                }
            } else if x > 1.0 {
                if self.shading.extend[1] {
                    x = 1.0;
                } else {
                    pixel.copy_from_slice(bg_color);
                    continue;
                }
            }

            let t = t0 + (t1 - t0) * x;
            
            let val = self.shading.function.eval(smallvec![t]).unwrap();

            let color = self.shading.color_space.to_rgba(&val, 1.0);
            pixel.copy_from_slice(&F::extract_color(&PremulColor::from_alpha_color(color)));

            pos += self.shading.y_advance;
        }
    }
}

impl Painter for AxialShadingFiller<'_> {
    fn paint<F: FineType>(self, target: &mut [F]) {
        self.run(target);
    }
}
