use crate::encode::{EncodeExt, EncodedPaint, Shader, x_y_advances};
use crate::fine::{COLOR_COMPONENTS, Sampler, from_rgba8};
use crate::paint::{Image, IndexedPaint, Paint};
use crate::pixmap::Pixmap;
use kurbo::{Affine, Point, Vec2};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct EncodedImage {
    pub(crate) pixmap: Arc<Pixmap>,
    pub(crate) interpolate: bool,
}

impl EncodedImage {
    fn get_pixmap_sample(&self, pos: Point) -> [f32; 4] {
        let mut x = pos.x as u16;
        let mut y = pos.y as u16;

        x = x.clamp(0, self.pixmap.width() - 1);
        y = y.clamp(0, self.pixmap.height() - 1);

        from_rgba8(&self.pixmap.sample(x, y).to_u8_array())
    }
}

impl EncodeExt for Image {
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint {
        let idx = paints.len();

        let encoded = EncodedImage {
            pixmap: self.pixmap.clone(),
            interpolate: self.interpolate,
        };

        let shader = Shader::new(transform.inverse(), encoded);

        if self.is_stencil {
            paints.push(EncodedPaint::Mask(shader));
        } else {
            paints.push(EncodedPaint::Image(shader));
        }

        Paint::Indexed(IndexedPaint::new(idx))
    }
}

impl Sampler for EncodedImage {
    fn sample(&self, pos: Point) -> [f32; 4] {
        if !self.interpolate {
            self.get_pixmap_sample(pos)
        } else {
            fn fract(val: f32) -> f32 {
                val - val.floor()
            }

            let x_fract = fract(pos.x as f32 + 0.5);
            let y_fract = fract(pos.y as f32 + 0.5);

            let mut interpolated_color = [0.0_f32; 4];

            let cx = [1.0 - x_fract, x_fract];
            let cy = [1.0 - y_fract, y_fract];

            for (x_idx, x) in [-0.5, 0.5].into_iter().enumerate() {
                for (y_idx, y) in [-0.5, 0.5].into_iter().enumerate() {
                    let color_sample = self.get_pixmap_sample(pos + Vec2::new(x, y));
                    let w = cx[x_idx] * cy[y_idx];

                    for (component, component_sample) in
                        interpolated_color.iter_mut().zip(color_sample)
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
}
