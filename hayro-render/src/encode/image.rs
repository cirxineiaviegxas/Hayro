use crate::encode::{EncodeExt, EncodedPaint, x_y_advances};
use crate::paint::{Image, IndexedPaint, Paint};
use crate::pixmap::Pixmap;
use kurbo::{Affine, Vec2};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct EncodedImage {
    pub(crate) pixmap: Arc<Pixmap>,
    pub(crate) interpolate: bool,
    pub(crate) repeat: bool,
    pub(crate) transform: Affine,
    pub(crate) x_advance: Vec2,
    pub(crate) y_advance: Vec2,
    pub(crate) is_stencil: bool,
}

impl EncodeExt for Image {
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint {
        let idx = paints.len();

        let transform = transform.inverse() * Affine::translate((0.5, 0.5));

        let (x_advance, y_advance) = x_y_advances(&transform);

        let encoded = EncodedImage {
            pixmap: self.pixmap.clone(),
            interpolate: self.interpolate,
            transform,
            repeat: self.repeat,
            x_advance,
            y_advance,
            is_stencil: self.is_stencil,
        };

        paints.push(EncodedPaint::Image(encoded));

        Paint::Indexed(IndexedPaint::new(idx))
    }
}
