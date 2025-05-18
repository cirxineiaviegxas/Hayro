use crate::encode::EncodedTriangleMeshShading;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use hayro_interpret::shading::{Triangle, TriangleVertex};
use kurbo::Point;
use peniko::color::palette::css::{BLACK, TRANSPARENT};
use smallvec::{ToSmallVec, smallvec};

#[derive(Debug)]
pub(crate) struct TriangleMeshShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedTriangleMeshShading,
    abort: bool,
}

impl<'a> TriangleMeshShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedTriangleMeshShading, start_x: u16, start_y: u16) -> Self {
        let cur_pos = shading.inverse_transform
            * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5);

        Self {
            cur_pos,
            shading,
            abort: start_y < 40,
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
            let mut filled = false;
            for triangle in &self.shading.triangles {
                if let Some(color) = interpolate_color(triangle, pos.x as f32, pos.y as f32) {
                    let color = if let Some(function) = &self.shading.function {
                        let val = function.eval(color.to_smallvec()).unwrap();
                        self.shading.color_space.to_rgba(&val, 1.0)
                    } else {
                        self.shading.color_space.to_rgba(&color, 1.0)
                    };

                    filled = true;
                    pixel.copy_from_slice(&PremulColor::from_alpha_color(color).0);
                }
            }

            if !filled {
                pixel.copy_from_slice(&PremulColor::from_alpha_color(TRANSPARENT).0);
            }
            pos += self.shading.y_advance;
        }
    }
}

impl Painter for TriangleMeshShadingFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}

fn interpolate_color(tri: &Triangle, px: f32, py: f32) -> Option<Vec<f32>> {
    let (u, v, w) = barycentric_coords(px, py, &tri.p0, &tri.p1, &tri.p2)?;

    if u < 0.0 || v < 0.0 || w < 0.0 {
        return None;
    }

    let mut result = Vec::with_capacity(tri.p0.colors.len());
    for i in 0..tri.p0.colors.len() {
        let c0 = tri.p0.colors[i];
        let c1 = tri.p1.colors[i];
        let c2 = tri.p2.colors[i];
        result.push(u * c0 + v * c1 + w * c2);
    }

    Some(result)
}

fn barycentric_coords(
    px: f32,
    py: f32,
    a: &TriangleVertex,
    b: &TriangleVertex,
    c: &TriangleVertex,
) -> Option<(f32, f32, f32)> {
    let v0 = (b.x - a.x, b.y - a.y);
    let v1 = (c.x - a.x, c.y - a.y);
    let v2 = (px - a.x, py - a.y);

    let dot00 = v0.0 * v0.0 + v0.1 * v0.1;
    let dot01 = v0.0 * v1.0 + v0.1 * v1.1;
    let dot02 = v0.0 * v2.0 + v0.1 * v2.1;
    let dot11 = v1.0 * v1.0 + v1.1 * v1.1;
    let dot12 = v1.0 * v2.0 + v1.1 * v2.1;

    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < f32::EPSILON {
        return None;
    }

    let inv_denom = 1.0 / denom;
    let mut v = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let mut w = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    let mut u = 1.0 - v - w;

    const EPSILON: f32 = -1e-4;
    
    if v >= EPSILON && v < 0.0 {
        v = 0.0;
    }
    
    if u >= EPSILON && u < 0.0 {
        u = 0.0;
    }
    
    if w >= EPSILON && w < 0.0 {
        w = 0.0;
    }

    if u >= 0.0 && v >= 0.0 && w >= 0.0 {
        Some((u, v, w))
    } else {
        None
    }
}
