// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paints for drawing shapes.

use crate::paint::{Image, IndexedPaint, Paint};
use crate::pixmap::Pixmap;
use kurbo::{Affine, Point, Vec2};
use peniko::ImageQuality;
use std::sync::Arc;

const DEGENERATE_THRESHOLD: f32 = 1.0e-6;
const NUDGE_VAL: f32 = 1.0e-7;

/// A trait for encoding gradients.
pub(crate) trait EncodeExt {
    /// Encode the gradient and push it into a vector of encoded paints, returning
    /// the corresponding paint in the process. This will also validate the gradient.
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint;
}

impl EncodeExt for Image {
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint {
        let idx = paints.len();

        let transform = transform.inverse();

        let (x_advance, y_advance) = x_y_advances(&transform);

        let encoded = EncodedImage {
            pixmap: self.pixmap.clone(),
            extends: (self.x_extend, self.y_extend),
            quality: self.quality,
            transform,
            x_advance,
            y_advance,
        };

        paints.push(EncodedPaint::Image(encoded));

        Paint::Indexed(IndexedPaint::new(idx))
    }
}

/// An encoded paint.
#[derive(Debug)]
pub enum EncodedPaint {
    /// An encoded image.
    Image(EncodedImage),
}

/// An encoded image.
#[derive(Debug)]
pub struct EncodedImage {
    /// The underlying pixmap of the image.
    pub pixmap: Arc<Pixmap>,
    /// The extends in the horizontal and vertical direction.
    pub extends: (peniko::Extend, peniko::Extend),
    /// The rendering quality of the image.
    pub quality: ImageQuality,
    /// A transform to apply to the image.
    pub transform: Affine,
    /// The advance in image coordinates for one step in the x direction.
    pub x_advance: Vec2,
    /// The advance in image coordinates for one step in the y direction.
    pub y_advance: Vec2,
}

pub(crate) fn x_y_advances(transform: &Affine) -> (Vec2, Vec2) {
    let scale_skew_transform = {
        let c = transform.as_coeffs();
        Affine::new([c[0], c[1], c[2], c[3], 0.0, 0.0])
    };

    let x_advance = scale_skew_transform * Point::new(1.0, 0.0);
    let y_advance = scale_skew_transform * Point::new(0.0, 1.0);

    (
        Vec2::new(x_advance.x, x_advance.y),
        Vec2::new(y_advance.x, y_advance.y),
    )
}
