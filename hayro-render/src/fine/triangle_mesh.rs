use crate::encode::EncodedTriangleMeshShading;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use hayro_interpret::color::ColorComponents;
use hayro_interpret::shading::Triangle;
use kurbo::{Point, Shape};
use smallvec::ToSmallVec;

#[derive(Debug)]
pub(crate) struct TriangleMeshShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedTriangleMeshShading,
    current: Option<&'a Triangle>,
}

impl<'a> TriangleMeshShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedTriangleMeshShading, start_x: u16, start_y: u16) -> Self {
        let cur_pos = shading.inverse_transform
            * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5);

        Self {
            cur_pos,
            shading,
            current: None,
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

    fn get_color(&mut self) -> Option<ColorComponents> {
        if let Some(triangle) = &mut self.current {
            if triangle.contains_point(self.cur_pos) {
                Some(triangle.interpolate(self.cur_pos))
            } else {
                self.update_current()
            }
        } else {
            self.update_current()
        }
    }

    fn update_current(&mut self) -> Option<ColorComponents> {
        // Do in reverse so that triangles that appear later in the stream are the ones that
        // will actually be painted.
        for triangle in self.shading.triangles.iter().rev() {
            if triangle.contains_point(self.cur_pos) {
                self.current = Some(triangle);

                return Some(triangle.interpolate(self.cur_pos));
            }
        }

        self.current = None;

        None
    }

    fn run_complex_column(&mut self, col: &mut [f32], bg_color: &[f32; 4]) {
        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            if let Some(color) = self.get_color() {
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

impl Painter for TriangleMeshShadingFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}
