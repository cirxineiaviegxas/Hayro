use crate::encode::EncodedSampledShading;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use kurbo::Point;
use smallvec::ToSmallVec;

#[derive(Debug)]
pub(crate) struct SampledShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedSampledShading,
}

impl<'a> SampledShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedSampledShading, start_x: u16, start_y: u16) -> Self {
        let cur_pos = shading.inverse_transform
            * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5);

        Self {
            cur_pos,
            shading,
        }
    }

    pub(super) fn run(mut self, target: &mut [f32]) {
        let bg_color = PremulColor::from_alpha_color(self.shading.background).0;

        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                let old_pos = self.cur_pos;
                self.run_complex_column(column, &bg_color);
                self.cur_pos = old_pos + self.shading.x_advance;
            });
    }

    fn run_complex_column(&mut self, col: &mut [f32], bg_color: &[f32; 4]) {
        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            let sample_point = (self.cur_pos.x as u16, self.cur_pos.y as u16);
            
            if let Some(color) = self.shading.samples.get(&sample_point) {
                let color = if let Some(function) = &self.shading.function {
                    let val = function.eval(&color.to_smallvec()).unwrap();
                    self.shading.color_space.to_rgba(&val, 1.0)
                } else {
                    self.shading.color_space.to_rgba(&color, 1.0)
                };

                pixel.copy_from_slice(&PremulColor::from_alpha_color(color).0);
            } else {
                pixel.copy_from_slice(bg_color);
            }

            self.cur_pos += self.shading.y_advance;
        }
    }
}

impl Painter for SampledShadingFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}
