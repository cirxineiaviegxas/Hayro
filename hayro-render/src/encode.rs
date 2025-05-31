// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paints for drawing shapes.

use crate::paint::{Image, IndexedPaint, Paint};
use crate::pixmap::Pixmap;
use hayro_interpret::color::{ColorComponents, ColorSpace};
use hayro_interpret::pattern::ShadingPattern;
use hayro_interpret::shading::{CoonsPatch, ShadingFunction, ShadingType, Triangle, TriangleVertex};
use kurbo::{Affine, Point, Vec2};
use peniko::ImageQuality;
use peniko::color::palette::css::TRANSPARENT;
use peniko::color::{AlphaColor, Srgb};
use std::sync::Arc;
use hayro_syntax::content::ops::Transform;
use rustc_hash::FxHashMap;

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
            } => encode_axial_shading(
                self, paints, transform, *coords, *domain, function, *extend, *axial,
            ),
            ShadingType::TriangleMesh {
                triangles,
                function,
            } => {
                let idx = paints.len();

                let full_transform = transform * self.matrix;
                let samples = sample_triangles(triangles, full_transform);
                
                let inverse_transform = Affine::IDENTITY;

                let (x_advance, y_advance) = x_y_advances(&inverse_transform);

                let cs = self.shading.color_space.clone();
                

                let encoded = EncodedSampledShading {
                    samples,
                    function: function.clone(),
                    x_advance,
                    y_advance,
                    color_space: cs.clone(),
                    inverse_transform,
                    background: self
                        .shading
                        .background
                        .as_ref()
                        .map(|b| cs.to_rgba(&b, 1.0))
                        .unwrap_or(TRANSPARENT),
                };

                paints.push(EncodedPaint::SampledShading(encoded));

                Paint::Indexed(IndexedPaint::new(idx))
            }
            ShadingType::CoonsPatchMesh { patches, function } => {
                let idx = paints.len();

                let full_transform = transform * self.matrix;

                let cs = self.shading.color_space.clone();

                let triangles = patches.iter().flat_map(|p| p.to_triangles()).collect::<Vec<_>>();
                let samples = sample_triangles(&triangles, full_transform);

                let inverse_transform = Affine::IDENTITY;

                let (x_advance, y_advance) = x_y_advances(&inverse_transform);

                let encoded = EncodedSampledShading {
                    samples,
                    function: function.clone(),
                    x_advance,
                    y_advance,
                    color_space: cs.clone(),
                    inverse_transform,
                    background: self
                        .shading
                        .background
                        .as_ref()
                        .map(|b| cs.to_rgba(&b, 1.0))
                        .unwrap_or(TRANSPARENT),
                };

                paints.push(EncodedPaint::SampledShading(encoded));

                Paint::Indexed(IndexedPaint::new(idx))
            }
            ShadingType::TensorProductPatchMesh { patches, function } => {
                let idx = paints.len();

                let full_transform = transform * self.matrix;

                let cs = self.shading.color_space.clone();

                let triangles = patches.iter().flat_map(|p| p.to_triangles()).collect::<Vec<_>>();
                let samples = sample_triangles(&triangles, full_transform);


                let inverse_transform = Affine::IDENTITY;

                let (x_advance, y_advance) = x_y_advances(&inverse_transform);
                

                let encoded = EncodedSampledShading {
                    samples,
                    function: function.clone(),
                    x_advance,
                    y_advance,
                    color_space: cs.clone(),
                    inverse_transform,
                    background: self
                        .shading
                        .background
                        .as_ref()
                        .map(|b| cs.to_rgba(&b, 1.0))
                        .unwrap_or(TRANSPARENT),
                };

                paints.push(EncodedPaint::SampledShading(encoded));

                Paint::Indexed(IndexedPaint::new(idx))
            }
        }
    }
}

fn encode_axial_shading(
    sp: &ShadingPattern,
    paints: &mut Vec<EncodedPaint>,
    transform: Affine,
    coords: [f32; 6],
    domain: [f32; 2],
    function: &ShadingFunction,
    extend: [bool; 2],
    is_axial: bool,
) -> Paint {
    let idx = paints.len();

    let mut p1 = Point::ZERO;
    let mut r = Point::ZERO;
    let initial_transform;

    let params = if is_axial {
        let [x_0, y_0, x_1, y_1, _, _] = coords;

        initial_transform = ts_from_line_to_line(
            Point::new(x_0 as f64, y_0 as f64), 
            Point::new(x_1 as f64, y_1 as f64), Point::ZERO, Point::new(1.0, 0.0));

        RadialAxialParams::Axial
    } else {
        let [x_0, y_0, r0, x_1, y_1, r_1] = coords;

        initial_transform = Affine::translate((-x_0 as f64, -y_0 as f64));
        let new_x1 = x_1 - x_0;
        let new_y1 = y_1 - y_0;

        p1 = Point::new(new_x1 as f64, new_y1 as f64);
        r = Point::new(r0 as f64, r_1 as f64);

        RadialAxialParams::Radial
    };

    let full_transform = transform * sp.matrix;
    let inverse_transform = initial_transform * full_transform.inverse();

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
        p1,
        r,
        domain,
        extend,
        axial: is_axial,
    };

    paints.push(EncodedPaint::AxialShading(encoded));

    Paint::Indexed(IndexedPaint::new(idx))
}

fn sample_triangles(triangles: &[Triangle], transform: Affine) -> FxHashMap<(u16, u16), ColorComponents> {
    let mut map = FxHashMap::default();
    
    for t in triangles {
        let t = {
            let p0 = transform * t.p0.point;
            let p1 = transform * t.p1.point;
            let p2 = transform * t.p2.point;
            
            let mut v0 = t.p0.clone();
            v0.point = p0; 
            let mut v1 = t.p1.clone();
            v1.point = p1;
            let mut v2 = t.p2.clone();
            v2.point = p2;
            
            Triangle::new(v0, v1, v2)
        };
        
        let bbox = t.bounding_box();
        
        for y in (bbox.y0.floor() as u16)..(bbox.y1.ceil() as u16) {
            for x in (bbox.x0.floor() as u16)..(bbox.x1.ceil() as u16) {
                let point = Point::new(x as f64, y as f64);
                    if t.contains_point(point) {
                        map.insert((x, y), t.interpolate(point));
                    }
            }
        }
    }
    
    map
}

fn encode_function_shading(
    sp: &ShadingPattern,
    paints: &mut Vec<EncodedPaint>,
    transform: Affine,
    domain: &[f32; 4],
    matrix: &Affine,
    function: &ShadingFunction,
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
    pub function: ShadingFunction,
    pub x_advance: Vec2,
    pub y_advance: Vec2,
    pub color_space: ColorSpace,
    pub background: AlphaColor<Srgb>,
}

#[derive(Debug)]
pub struct EncodedTriangleMeshShading {
    pub triangles: Vec<Triangle>,
    pub function: Option<ShadingFunction>,
    pub x_advance: Vec2,
    pub y_advance: Vec2,
    pub color_space: ColorSpace,
    pub inverse_transform: Affine,
    pub background: AlphaColor<Srgb>,
}

#[derive(Debug)]
pub enum RadialAxialParams {
    Axial,
    Radial,
}

#[derive(Debug)]
pub struct EncodedRadialAxialShading {
    pub inverse_transform: Affine,
    pub function: ShadingFunction,
    pub x_advance: Vec2,
    pub y_advance: Vec2,
    pub color_space: ColorSpace,
    pub background: AlphaColor<Srgb>,
    pub params: RadialAxialParams,
    pub p1: Point,
    pub r: Point,
    pub domain: [f32; 2],
    pub extend: [bool; 2],
    pub axial: bool,
}

#[derive(Debug)]
pub struct EncodedSampledShading {
    pub samples: FxHashMap<(u16, u16), ColorComponents>,
    pub function: Option<ShadingFunction>,
    pub x_advance: Vec2,
    pub y_advance: Vec2,
    pub color_space: ColorSpace,
    pub inverse_transform: Affine,
    pub background: AlphaColor<Srgb>,
}

/// An encoded paint.
#[derive(Debug)]
pub enum EncodedPaint {
    /// An encoded image.
    Image(EncodedImage),
    FunctionShading(EncodedFunctionShading),
    AxialShading(EncodedRadialAxialShading),
    SampledShading(EncodedSampledShading),
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

fn ts_from_line_to_line(src1: Point, src2: Point, dst1: Point, dst2: Point) -> Affine {
    let unit_to_line1 = unit_to_line(src1, src2);
    // Calculate the transform necessary to map line1 to the unit vector.
    let line1_to_unit = unit_to_line1.inverse();
    // Then map the unit vector to line2.
    let unit_to_line2 = unit_to_line(dst1, dst2);

    unit_to_line2 * line1_to_unit
}

fn unit_to_line(p0: Point, p1: Point) -> Affine {
    Affine::new([
        p1.y - p0.y,
        p0.x - p1.x,
        p1.x - p0.x,
        p1.y - p0.y,
        p0.x,
        p0.y,
    ])
}
