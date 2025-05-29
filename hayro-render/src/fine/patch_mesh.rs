use crate::encode::EncodedPatchMeshShading;
use crate::fine::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use crate::paint::PremulColor;
use hayro_interpret::shading::CoonsPatch;
use kurbo::{CubicBez, ParamCurve, Point};
use smallvec::ToSmallVec;

#[derive(Debug)]
pub(crate) struct PatchMeshShadingFiller<'a> {
    cur_pos: Point,
    shading: &'a EncodedPatchMeshShading,
}

impl<'a> PatchMeshShadingFiller<'a> {
    pub(crate) fn new(shading: &'a EncodedPatchMeshShading, start_x: u16, start_y: u16) -> Self {
        let cur_pos = shading.inverse_transform
            * Point::new(f64::from(start_x) + 0.5, f64::from(start_y) + 0.5);

        Self { cur_pos, shading }
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
            let mut color_found = false;

            for patch in &self.shading.patches {
                if let Some((u, v)) = find_uv(patch, self.cur_pos) {
                    let t_or_color = bilinear_color(patch, u, v);

                    let final_color = if let Some(function) = &self.shading.function {
                        let val = function.eval(&t_or_color.to_smallvec()).unwrap();
                        self.shading.color_space.to_rgba(&val, 1.0)
                    } else {
                        self.shading.color_space.to_rgba(&t_or_color, 1.0)
                    };

                    pixel.copy_from_slice(&PremulColor::from_alpha_color(final_color).0);
                    color_found = true;
                    break;
                }
            }

            if !color_found {
                pixel.copy_from_slice(bg_color);
            }

            self.cur_pos += self.shading.y_advance;
        }
    }
}

impl Painter for PatchMeshShadingFiller<'_> {
    fn paint(self, target: &mut [f32]) {
        self.run(target);
    }
}

fn bilinear_color(patch: &CoonsPatch, u: f64, v: f64) -> Vec<f32> {
    let (c0, c1, c2, c3) = {
        (
            &patch.colors[0],
            &patch.colors[1],
            &patch.colors[2],
            &patch.colors[3],
        )
    };

    let mut result = Vec::new();
    for i in 0..c0.len() {
        let val = (1.0 - u) * (1.0 - v) * c0[i] as f64
            + u * (1.0 - v) * c3[i] as f64
            + u * v * c2[i] as f64
            + (1.0 - u) * v * c1[i] as f64;
        result.push(val as f32);
    }

    result
}

fn find_uv(patch: &CoonsPatch, target: Point) -> Option<(f64, f64)> {
    let mut best = None;
    let mut min_dist = f64::MAX;

    const GRANULARITY: usize = 20;

    for i in 0..=GRANULARITY {
        for j in 0..=GRANULARITY {
            let u = i as f64 / GRANULARITY as f64;
            let v = j as f64 / GRANULARITY as f64;
            let s = eval_patch_surface(patch, u, v);
            let dist = (s - target).hypot();
            if dist < min_dist {
                min_dist = dist;
                best = Some((u, v));
            }
        }
    }

    // TODO: Try to understand + improve this.
    // TODO: There still are some small artifacts in the cornners
    let (mut u, mut v) = best?;

    for _ in 0..10 {
        let s = eval_patch_surface(patch, u, v);
        let diff = s - target;

        let epsilon = 1e-5;
        let s_u = (eval_patch_surface(patch, u + epsilon, v) - s) / epsilon;
        let s_v = (eval_patch_surface(patch, u, v + epsilon) - s) / epsilon;

        let det = s_u.x * s_v.y - s_u.y * s_v.x;
        if det.abs() < 1e-8 {
            return None;
        }

        let inv_jacobian = (
            Point::new(s_v.y / det, -s_u.y / det),
            Point::new(-s_v.x / det, s_u.x / det),
        );

        let delta_u = inv_jacobian.0.x * diff.x + inv_jacobian.1.x * diff.y;
        let delta_v = inv_jacobian.0.y * diff.x + inv_jacobian.1.y * diff.y;

        u -= delta_u;
        v -= delta_v;

        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return None;
        }

        if diff.hypot() < 0.25 {
            break;
        }
    }

    let final_pos = eval_patch_surface(patch, u, v);
    let distance = (final_pos - target).hypot();

    if distance < 1.0 { Some((u, v)) } else { None }
}

fn eval_patch_surface(patch: &CoonsPatch, u: f64, v: f64) -> Point {
    let cp = &patch.control_points;

    let c1 = CubicBez::new(cp[0], cp[11], cp[10], cp[9]);
    let c2 = CubicBez::new(cp[3], cp[4], cp[5], cp[6]);
    let d1 = CubicBez::new(cp[0], cp[1], cp[2], cp[3]);
    let d2 = CubicBez::new(cp[9], cp[8], cp[7], cp[6]);

    let sc = (1.0 - v) * c1.eval(u).to_vec2() + v * c2.eval(u).to_vec2();
    let sd = (1.0 - u) * d1.eval(v).to_vec2() + u * d2.eval(v).to_vec2();
    let sb = (1.0 - v) * ((1.0 - u) * c1.eval(0.0).to_vec2() + u * c1.eval(1.0).to_vec2())
        + v * ((1.0 - u) * c2.eval(0.0).to_vec2() + u * c2.eval(1.0).to_vec2());

    (sc + sd - sb).to_point()
}
