use crate::encode::EncodedTriangleMeshShading;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use hayro_interpret::shading::{Triangle, TriangleVertex};
use kurbo::Point;
use peniko::color::palette::css::BLACK;
use smallvec::{ToSmallVec, smallvec};

#[derive(Debug)]
pub(crate) struct TriangleMeshShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedTriangleMeshShading,
}

impl<'a> TriangleMeshShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedTriangleMeshShading, start_x: u16, start_y: u16) -> Self {
        let cur_pos = shading.inverse_transform
            * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5);

        Self { cur_pos, shading }
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
            for triangle in &self.shading.triangles {
                if let Some(color) = interpolate_color(triangle, pos.x as f32, pos.y as f32) {
                    let color = if let Some(function) = &self.shading.function {
                        let val = function.eval(color.to_smallvec()).unwrap();
                        self.shading.color_space.to_rgba(&val, 1.0)
                    } else {
                        self.shading.color_space.to_rgba(&color, 1.0)
                    };

                    pixel.copy_from_slice(&PremulColor::from_alpha_color(color).0);
                }
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

    let mut result = vec![];
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
    let v0x = b.x - a.x;
    let v0y = b.y - a.y;
    let v1x = c.x - a.x;
    let v1y = c.y - a.y;
    let v2x = px - a.x;
    let v2y = py - a.y;

    let d00 = v0x * v0x + v0y * v0y;
    let d01 = v0x * v1x + v0y * v1y;
    let d11 = v1x * v1x + v1y * v1y;
    let d20 = v2x * v0x + v2y * v0y;
    let d21 = v2x * v1x + v2y * v1y;

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < f32::EPSILON {
        return None; // Degenerate triangle
    }

    let inv_denom = 1.0 / denom;
    let v = (d11 * d20 - d01 * d21) * inv_denom;
    let w = (d00 * d21 - d01 * d20) * inv_denom;
    let u = 1.0 - v - w;

    if u >= 0.0 && v >= 0.0 && w >= 0.0 {
        Some((u, v, w)) // Inside triangle
    } else {
        None
    }
}
