// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub(crate) mod image;
pub(crate) mod shading;

use crate::encode::image::EncodedImage;
use crate::encode::shading::EncodedShading;
use crate::fine::Sampler;
use crate::paint::Paint;
use kurbo::{Affine, Point, Vec2};

#[derive(Debug)]
pub struct Shader<T: Sampler> {
    pub(crate) transform: Affine,
    pub(crate) x_advance: Vec2,
    pub(crate) y_advance: Vec2,
    pub(crate) sampler: T,
}

impl<T: Sampler> Shader<T> {
    pub(crate) fn new(transform: Affine, sampler: T) -> Shader<T> {
        let transform = transform * Affine::translate((0.5, 0.5));
        let (x_advance, y_advance) = x_y_advances(&transform);

        Shader {
            transform,
            x_advance,
            y_advance,
            sampler,
        }
    }

    pub fn sample(&self, pos: Point) -> [f32; 4] {
        self.sampler.sample(pos)
    }
}

pub(crate) trait EncodeExt {
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint;
}

#[derive(Debug)]
pub enum EncodedPaint {
    Image(Shader<EncodedImage>),
    Mask(Shader<EncodedImage>),
    Shading(Shader<EncodedShading>),
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
