// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub(crate) mod image;
pub(crate) mod shading;

use kurbo::{Affine, Point, Vec2};
use crate::encode::image::EncodedImage;
use crate::encode::shading::EncodedShading;
use crate::paint::Paint;

pub(crate) trait EncodeExt {
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint;
}

#[derive(Debug)]
pub enum EncodedPaint {
    Image(EncodedImage),
    Shading(EncodedShading),
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
