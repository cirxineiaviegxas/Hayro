// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use kurbo::Point;
use crate::encode::shading::EncodedShading;

#[derive(Debug)]
pub(crate) struct ShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedShading,
}

impl<'a> ShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedShading, start_x: u16, start_y: u16) -> Self {
        Self {
            cur_pos: shading.initial_transform
                * Point::new(f64::from(start_x), f64::from(start_y)),
            shading,
        }
    }

    pub(super) fn run(mut self, target: &mut [f32]) {
        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                let mut pos = self.cur_pos;

                for pixel in column.chunks_exact_mut(COLOR_COMPONENTS) {
                    let color = self.shading.shading_type.eval(pos, self.shading.background_color, &self.shading.color_space);
                    pixel.copy_from_slice(&PremulColor::from_alpha_color(color).0);
                    
                    pos += self.shading.y_advance;
                }
                
                self.cur_pos += self.shading.x_advance;
            });
    }
}

impl Painter for ShadingFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}