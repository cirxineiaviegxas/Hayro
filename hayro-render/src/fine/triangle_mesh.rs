use crate::encode::EncodedTriangleMeshShading;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use hayro_interpret::shading::{Triangle};
use kurbo::Point;
use smallvec::{ToSmallVec};

#[derive(Debug)]
pub(crate) struct TriangleMeshShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedTriangleMeshShading,
    current: Option<(&'a Triangle, Vec<f32>)>,
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

    fn get_color(&mut self) -> Option<Vec<f32>> {
        if let Some((tri, color)) = &mut self.current {
            if let Some(col) = interpolate_color(*tri, self.cur_pos) {
                *color = col;
            } else {
                self.update_current();
            }
        } else {
            self.update_current();
        }

        self.current.as_ref().map(|e| e.1.clone())
    }

    fn update_current(&mut self) {
        for triangle in &self.shading.triangles {
            if let Some(color) = interpolate_color(triangle, self.cur_pos) {
                self.current = Some((triangle, color));

                return;
            }
        }

        self.current = None;
    }

    fn run_complex_column(&mut self, col: &mut [f32], bg_color: &[f32; 4]) {
        for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
            if let Some(color) = self.get_color() {
                let color = if let Some(function) = &self.shading.function {
                    let val = function.eval(color.to_smallvec()).unwrap();
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

fn interpolate_color(tri: &Triangle, pos: Point) -> Option<Vec<f32>> {
    let (u, v, w) = barycentric_coords(pos, &tri)?;

    let mut result = Vec::with_capacity(tri.p0.colors.len());
    for i in 0..tri.p0.colors.len() {
        let c0 = tri.p0.colors[i];
        let c1 = tri.p1.colors[i];
        let c2 = tri.p2.colors[i];
        result.push(u * c0 + v * c1 + w * c2);
    }

    Some(result)
}

fn barycentric_coords(p: Point, tri: &Triangle) -> Option<(f32, f32, f32)> {
    let (a, b, c) = (tri.p0.point, tri.p1.point, tri.p2.point);
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let nudge = |val: f64| -> Option<f64> {
        const EPSILON: f64 = -1e-4;

        if val < EPSILON {
            None
        } else {
            Some(val.max(0.0))
        }
    };

    let denom = d00 * d11 - d01 * d01;
    let v = nudge((d11 * d20 - d01 * d21) / denom)?;
    let w = nudge((d00 * d21 - d01 * d20) / denom)?;
    let u = nudge(1.0 - v - w)? as f32;

    Some((u, v as f32, w as f32))
}
