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
    fn interpolate(&self) -> bool {
        self.interpolate
    }

    fn sample_impl(&self, pos: Point) -> [f32; 4] {
        let mut x = pos.x as u16;
        let mut y = pos.y as u16;

        x = x.clamp(0, self.pixmap.width() - 1);
        y = y.clamp(0, self.pixmap.height() - 1);

        from_rgba8(&self.pixmap.sample(x, y).to_u8_array())
    }
}
