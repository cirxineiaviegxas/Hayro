// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::encode::{
    EncodedFunctionShading, EncodedRadialAxialShading, EncodedTriangleMeshShading,
    RadialAxialParams,
};
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use hayro_interpret::shading::Triangle;
use kurbo::Point;
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

    pub(super) fn run(mut self, target: &mut [f32]) {
        let bg_color = PremulColor::from_alpha_color(self.shading.background).0;

        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                self.run_complex_column(column, &bg_color);
                self.cur_pos += self.shading.x_advance;
            });
    }

    fn run_complex_column(&mut self, col: &mut [f32], bg_color: &[f32; 4]) {
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
                pixel.copy_from_slice(&PremulColor::from_alpha_color(color).0);
            }
            pos += self.shading.y_advance;
        }
    }
}

impl Painter for FunctionShadingFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}

#[derive(Debug)]
pub(crate) struct RadialAxialShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedRadialAxialShading,
}

impl<'a> RadialAxialShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedRadialAxialShading, start_x: u16, start_y: u16) -> Self {
        Self {
            // We want to sample values of the pixels at the center, so add an offset of 0.5.
            cur_pos: shading.inverse_transform
                * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5),
            shading,
        }
    }

    pub(super) fn run(mut self, target: &mut [f32]) {
        let bg_color = PremulColor::from_alpha_color(self.shading.background).0;

        let denom = match self.shading.params {
            RadialAxialParams::Axial { denom } => denom,
            RadialAxialParams::Radial { .. } => 0.0,
        };

        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                self.fill_axial(column, &bg_color, denom);
                self.cur_pos += self.shading.x_advance;
            });
    }

    fn fill_axial(&mut self, col: &mut [f32], bg_color: &[f32; 4], denom: f32) {
        // TODO: If the
        // starting and ending coordinates are coincident (x0=x1 and y0=y1) nothing shall be
        // painted.

        let mut pos = self.cur_pos;
        let (x1, y1) = (self.shading.p1.x as f32, self.shading.p1.y as f32);

        let (t0, t1) = (self.shading.domain[0], self.shading.domain[1]);

        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            let mut x = if self.shading.axial {
                let (x, y) = (pos.x as f32, pos.y as f32);
                // Note that x0 is not needed because we shortened it to 0.
                let p1 = x1 * x + y1 * y;

                p1 / denom
            } else {
                radial_pos(
                    &pos,
                    &self.shading.p1,
                    self.shading.r,
                    self.shading.extend[0],
                    self.shading.extend[1],
                )
                .unwrap_or(f32::MIN)
            };

            if x == f32::MIN {
                pixel.copy_from_slice(bg_color);

                pos += self.shading.y_advance;
                continue;
            }

            if x < 0.0 {
                if self.shading.extend[0] {
                    x = 0.0;
                } else {
                    pos += self.shading.y_advance;
                    pixel.copy_from_slice(bg_color);
                    continue;
                }
            } else if x > 1.0 {
                if self.shading.extend[1] {
                    x = 1.0;
                } else {
                    pos += self.shading.y_advance;
                    pixel.copy_from_slice(bg_color);
                    continue;
                }
            }

            let t = t0 + (t1 - t0) * x;

            let val = self.shading.function.eval(smallvec![t]).unwrap();

            let color = self.shading.color_space.to_rgba(&val, 1.0);
            pixel.copy_from_slice(&PremulColor::from_alpha_color(color).0);

            pos += self.shading.y_advance;
        }
    }
}

fn radial_pos(
    pos: &Point,
    p1: &Point,
    r: Point,
    min_extend: bool,
    max_extend: bool,
) -> Option<f32> {
    // The values for a radial gradient can be calculated for any t as follow:
    // Let x(t) = (x_1 - x_0)*t + x_0 (since x_0 is always 0, this shortens to x_1 * t)
    // Let y(t) = (y_1 - y_0)*t + y_0 (since y_0 is always 0, this shortens to y_1 * t)
    // Let r(t) = (r_1 - r_0)*t + r_0
    // Given a pixel at a position (x_2, y_2), we need to find the largest t such that
    // (x_2 - x(t))^2 + (y - y_(t))^2 = r_t()^2, i.e. the circle with the interpolated
    // radius and center position needs to intersect the pixel we are processing.
    //
    // You can reformulate this problem to a quadratic equation (TODO: add derivation. Since
    // I'm not sure if that code will stay the same after performance optimizations I haven't
    // written this down yet), to which we then simply need to find the solutions.

    let r0 = r.x as f32;
    let dx = p1.x as f32;
    let dy = p1.y as f32;
    let dr = r.y as f32 - r0;

    let px = pos.x as f32;
    let py = pos.y as f32;

    let a = dx * dx + dy * dy - dr * dr;
    let b = -2.0 * (px * dx + py * dy + r0 * dr);
    let c = px * px + py * py - r0 * r0;

    let discriminant = b * b - 4.0 * a * c;

    // No solution available.
    if discriminant < 0.0 {
        return None;
    }

    if a.abs() < 1e-6 {
        if b.abs() < 1e-6 {
            return None;
        }

        let t = -c / b;

        if (!min_extend && t < 0.0) || (!max_extend && t > 1.0) {
            return None;
        }

        let r_t = r0 + dr * t;
        if r_t < 0.0 {
            return None;
        }

        return Some(t);
    }

    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);

    let max = t1.max(t2);
    let mut take_max = Some(max);
    let min = t1.min(t2);
    let mut take_min = Some(min);

    if (!min_extend && min < 0.0) || r0 + dr * min < 0.0 {
        take_min = None;
    }

    if (!max_extend && max > 1.0) || r0 + dr * max < 0.0 {
        take_max = None;
    }

    match (take_min, take_max) {
        (Some(_), Some(max)) => Some(max),
        (Some(min), None) => Some(min),
        (None, Some(max)) => Some(max),
        (None, None) => None,
    }
}

impl Painter for RadialAxialShadingFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}
