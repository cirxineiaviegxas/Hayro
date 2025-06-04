use crate::encode::{EncodeExt, EncodedPaint, Shader, x_y_advances, Buffer};
use crate::fine::{COLOR_COMPONENTS, Sampler, from_rgba8};
use crate::paint::{Image, IndexedPaint, Paint};
use kurbo::{Affine, Point, Vec2};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct EncodedImage {
    pub(crate) buffer: Arc<Buffer<4>>,
    pub(crate) interpolate: bool,
}

impl EncodeExt for Image {
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint {
        let idx = paints.len();

        let encoded = EncodedImage {
            buffer: self.buffer.clone(),
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
    fn interpolate(&self) -> bool {
        self.interpolate
    }

    fn sample_impl(&self, mut pos: Point) -> [f32; 4] {
        pos.x = pos.x.clamp(0.0, self.buffer.width as f64 - 1.0);
        pos.y = pos.y.clamp(0.0, self.buffer.height as f64 - 1.0);

        self.buffer.sample(pos)
    }
}
