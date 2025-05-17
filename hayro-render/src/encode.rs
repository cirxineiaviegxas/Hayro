// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paints for drawing shapes.

use crate::paint::{Image, IndexedPaint, Paint};
use crate::pixmap::Pixmap;
use hayro_interpret::color::ColorSpace;
use hayro_interpret::pattern::ShadingPattern;
use hayro_interpret::shading::ShadingType;
use hayro_syntax::function::Function;
use hayro_syntax::object::rect::Rect;
use kurbo::{Affine, Point, Vec2};
use peniko::ImageQuality;
use peniko::color::palette::css::{GREEN, TRANSPARENT};
use peniko::color::{AlphaColor, Srgb};
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
            x_step: self.x_step,
            y_step: self.y_step,
            is_stencil: self.is_stencil,
        };

        paints.push(EncodedPaint::Image(encoded));

        Paint::Indexed(IndexedPaint::new(idx))
    }
}

impl EncodeExt for ShadingPattern {
    fn encode_into(&self, paints: &mut Vec<EncodedPaint>, transform: Affine) -> Paint {
        match self.shading.shading_type.as_ref() {
            ShadingType::FunctionBased {
                domain,
                matrix,
                function,
            } => encode_function_shading(self, paints, transform, domain, matrix, function),
            ShadingType::RadialAxial {
                coords,
                domain,
                function,
                extend,
                axial,
            } => encode_axial_shading(self, paints, transform, *coords, *domain, function, *extend, *axial),
            _ => unimplemented!(),
        }
    }
}

fn encode_axial_shading(
    sp: &ShadingPattern,
    paints: &mut Vec<EncodedPaint>,
    transform: Affine,
    coords: [f32; 6],
    domain: [f32; 2],
    function: &Function,
    extend: [bool; 2],
    is_axial: bool,
) -> Paint {
    let idx = paints.len();
    
    let mut p0;
    let mut p1;
    let mut r;
    
    let params = if is_axial {
        let [x_0, y_0, x_1, y_1, _, _] = coords;
        
        p0 = Point::new(x_0 as f64, y_0 as f64);
        p1 = Point::new(x_1 as f64, y_1 as f64);
        r = Point::default();
        
        RadialAxialParams::Axial {denom: (x_1 - x_0) * (x_1 - x_0) + (y_1 - y_0) * (y_1 - y_0)}
    }   else {
        let [x_0, y_0, r0, x_1, y_1, r_1] = coords;

        p0 = Point::new(x_0 as f64, y_0 as f64);
        p1 = Point::new(x_1 as f64, y_1 as f64);
        r = Point::new(r0 as f64, r_1 as f64);
        
        RadialAxialParams::Radial
    };
    
    let full_transform = transform * sp.matrix;
    let inverse_transform = full_transform.inverse();

    let (x_advance, y_advance) = x_y_advances(&inverse_transform);

    let cs = sp.shading.color_space.clone();

    let encoded = EncodedRadialAxialShading {
        inverse_transform,
        x_advance,
        y_advance,
        function: function.clone(),
        color_space: cs.clone(),
        background: sp
            .shading
            .background
            .as_ref()
            .map(|b| cs.to_rgba(&b, 1.0))
            .unwrap_or(TRANSPARENT),
        params,
        p0,
        p1,
        r,
        domain,
        extend,
    };

    paints.push(EncodedPaint::AxialShading(encoded));

    Paint::Indexed(IndexedPaint::new(idx))
}

fn encode_function_shading(
    sp: &ShadingPattern,
    paints: &mut Vec<EncodedPaint>,
    transform: Affine,
    domain: &[f32; 4],
    matrix: &Affine,
    function: &Function,
) -> Paint {
    let idx = paints.len();

    let shading_transform = *matrix;

    let full_transform = transform * sp.matrix * shading_transform;
    let inverse_transform = full_transform.inverse();

    let (x_advance, y_advance) = x_y_advances(&inverse_transform);

    let cs = sp.shading.color_space.clone();

    let d = kurbo::Rect::new(
        domain[0] as f64,
        domain[2] as f64,
        domain[1] as f64,
        domain[3] as f64,
    );
    let encoded = EncodedFunctionShading {
        domain: d,
        inverse_transform,
        x_advance,
        y_advance,
        function: function.clone(),
        color_space: cs.clone(),
        background: sp
            .shading
            .background
            .as_ref()
            .map(|b| cs.to_rgba(&b, 1.0))
            .unwrap_or(TRANSPARENT),
    };

    paints.push(EncodedPaint::FunctionShading(encoded));

    Paint::Indexed(IndexedPaint::new(idx))
}

#[derive(Debug)]
pub struct EncodedFunctionShading {
    pub domain: kurbo::Rect,
    pub inverse_transform: Affine,
    pub function: Function,
    pub x_advance: Vec2,
    pub y_advance: Vec2,
    pub color_space: ColorSpace,
    pub background: AlphaColor<Srgb>,
}

#[derive(Debug)]
pub enum RadialAxialParams {
    Axial {
        denom: f32,
    },
    Radial
}

#[derive(Debug)]
pub struct EncodedRadialAxialShading {
    pub inverse_transform: Affine,
    pub function: Function,
    pub x_advance: Vec2,
    pub y_advance: Vec2,
    pub color_space: ColorSpace,
    pub background: AlphaColor<Srgb>,
    pub params: RadialAxialParams,
    pub p0: Point,
    pub p1: Point,
    pub r: Point,
    pub domain: [f32; 2],
    pub extend: [bool; 2],
}

/// An encoded paint.
#[derive(Debug)]
pub enum EncodedPaint {
    /// An encoded image.
    Image(EncodedImage),
    FunctionShading(EncodedFunctionShading),
    AxialShading(EncodedRadialAxialShading),
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
    pub x_step: f32,
    pub y_step: f32,
    pub is_stencil: bool,
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
