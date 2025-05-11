use crate::util::OptionLog;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BLACK_POINT, GAMMA, MATRIX, N, RANGE, WHITE_POINT};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::name::names::*;
use hayro_syntax::object::stream::Stream;
use hayro_syntax::object::{Object, string};
use log::warn;
use once_cell::sync::Lazy;
use peniko::color::{AlphaColor, Srgb};
use qcms::Transform;
use smallvec::SmallVec;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub(crate) type ColorComponents = SmallVec<[f32; 4]>;

#[derive(Debug, Clone)]
pub(crate) enum ColorSpace {
    DeviceCmyk,
    DeviceGray,
    DeviceRgb,
    Indexed(Indexed),
    ICCColor(ICCProfile),
    CalGray(CalGray),
    CalRgb(CalRgb),
    Lab(Lab),
}

impl ColorSpace {
    pub fn new(object: Object) -> ColorSpace {
        Self::new_inner(object)
            .warn_none("unsupported color space or failed to process it")
            .unwrap_or(ColorSpace::DeviceGray)
    }

    fn new_inner(object: Object) -> Option<ColorSpace> {
        if let Ok(name) = object.clone().cast::<Name>() {
            return Self::new_from_name(name.clone());
        } else if let Ok(color_array) = object.clone().cast::<Array>() {
            let mut iter = color_array.clone().iter::<Object>();
            let name = iter.next()?.cast::<Name>().ok()?;

            match name.as_ref() {
                ICC_BASED => {
                    let icc_stream = iter.next()?.cast::<Stream>().ok()?;
                    let num_components = icc_stream.dict().get::<usize>(N)?;
                    let profile =
                        ICCProfile::new(icc_stream.decoded().ok()?.as_ref(), num_components)?;
                    return Some(ColorSpace::ICCColor(profile));
                    // TODO: How to handle range?
                    // TODO: Handle alternate.
                }
                CAL_CMYK => return Some(ColorSpace::DeviceCmyk),
                CAL_GRAY => {
                    let cal_dict = iter.next()?.cast::<Dict>().ok()?;
                    return Some(ColorSpace::CalGray(CalGray::new(&cal_dict)?));
                }
                CAL_RGB => {
                    let cal_dict = iter.next()?.cast::<Dict>().ok()?;
                    return Some(ColorSpace::CalRgb(CalRgb::new(&cal_dict)?));
                }
                LAB => {
                    let lab_dict = iter.next()?.cast::<Dict>().ok()?;
                    return Some(ColorSpace::Lab(Lab::new(&lab_dict)?));
                }
                INDEXED => return Some(ColorSpace::Indexed(Indexed::new(&color_array)?)),
                _ => {
                    warn!("unsupported color space: {}", name.as_str());
                    return None;
                },
            }
        }

        None
    }

    pub fn new_from_name(name: Name) -> Option<ColorSpace> {
        match name.as_ref() {
            DEVICE_RGB | RGB => Some(ColorSpace::DeviceRgb),
            DEVICE_GRAY | G => Some(ColorSpace::DeviceGray),
            DEVICE_CMYK | CMYK => Some(ColorSpace::DeviceCmyk),
            CAL_CMYK => Some(ColorSpace::DeviceCmyk),
            PATTERN => {
                warn!("pattern color spaces are not supported yet");

                Some(ColorSpace::DeviceGray)
            }
            _ => None,
        }
    }

    pub fn default_decode_arr(&self) -> Vec<(f32, f32)> {
        match self {
            ColorSpace::DeviceCmyk => vec![(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
            ColorSpace::DeviceGray => vec![(0.0, 1.0)],
            ColorSpace::DeviceRgb => vec![(0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
            ColorSpace::ICCColor(i) => vec![(0.0, 1.0); i.0.number_components],
            ColorSpace::CalGray(_) => vec![(0.0, 1.0)],
            ColorSpace::CalRgb(_) => vec![(0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
            ColorSpace::Lab(l) => vec![
                (0.0, 100.0),
                (l.0.range[0], l.0.range[1]),
                (l.0.range[2], l.0.range[3]),
            ],
            ColorSpace::Indexed(_) => vec![(0.0, 255.0)],
        }
    }

    pub fn set_initial_color(&self, components: &mut ColorComponents) {
        components.truncate(0);

        match self {
            ColorSpace::DeviceCmyk => components.extend([0.0, 0.0, 0.0, 1.0]),
            ColorSpace::DeviceGray => components.push(0.0),
            ColorSpace::DeviceRgb => components.extend([0.0, 0.0, 0.0]),
            ColorSpace::ICCColor(icc) => match icc.0.number_components {
                1 => components.push(0.0),
                3 => components.extend([0.0, 0.0, 0.0]),
                4 => components.extend([0.0, 0.0, 0.0, 1.0]),
                _ => unreachable!(),
            },
            ColorSpace::CalGray(_) => components.push(0.0),
            ColorSpace::CalRgb(_) => components.extend([0.0, 0.0, 0.0]),
            ColorSpace::Lab(_) => components.extend([0.0, 0.0, 0.0]),
            ColorSpace::Indexed(_) => components.push(0.0),
        }
    }

    pub fn components(&self) -> u8 {
        match self {
            ColorSpace::DeviceCmyk => 4,
            ColorSpace::DeviceGray => 1,
            ColorSpace::DeviceRgb => 3,
            ColorSpace::ICCColor(icc) => icc.0.number_components as u8,
            ColorSpace::CalGray(_) => 1,
            ColorSpace::CalRgb(_) => 3,
            ColorSpace::Lab(_) => 3,
            ColorSpace::Indexed(_) => 1,
        }
    }

    pub fn to_rgba(&self, c: &[f32], opacity: f32) -> AlphaColor<Srgb> {
        match &self {
            ColorSpace::DeviceRgb => AlphaColor::new([c[0], c[1], c[2], opacity]),
            ColorSpace::DeviceGray => AlphaColor::new([c[0], c[0], c[0], opacity]),
            ColorSpace::DeviceCmyk => {
                let opacity = u8_to_f32(opacity);
                let srgb = CMYK_TRANSFORM.to_rgba(&c[..]);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorSpace::ICCColor(icc) => {
                let opacity = u8_to_f32(opacity);
                let srgb = icc.to_rgba(&c[..]);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorSpace::CalGray(cal) => {
                let opacity = u8_to_f32(opacity);
                let srgb = cal.to_rgb(c[0]);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorSpace::CalRgb(cal) => {
                let opacity = u8_to_f32(opacity);
                let srgb = cal.to_rgb([c[0], c[1], c[2]]);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorSpace::Lab(lab) => {
                let opacity = u8_to_f32(opacity);
                let srgb = lab.to_rgb([c[0], c[1], c[2]]);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorSpace::Indexed(i) => i.to_rgb(c[0], opacity),
        }
    }
}

#[derive(Debug)]
struct CalGrayRepr {
    white_point: [f32; 3],
    black_point: [f32; 3],
    gamma: f32,
}

#[derive(Debug, Clone)]
pub struct CalGray(Arc<CalGrayRepr>);

// See <https://github.com/mozilla/pdf.js/blob/06f44916c8936b92f464d337fe3a0a6b2b78d5b4/src/core/colorspace.js#L752>
impl CalGray {
    pub fn new(dict: &Dict) -> Option<Self> {
        let white_point = dict.get::<[f32; 3]>(WHITE_POINT).unwrap_or([1.0, 1.0, 1.0]);
        let black_point = dict.get::<[f32; 3]>(BLACK_POINT).unwrap_or([0.0, 0.0, 0.0]);
        let gamma = dict.get::<f32>(GAMMA).unwrap_or(1.0);

        Some(Self(Arc::new(CalGrayRepr {
            white_point,
            black_point,
            gamma,
        })))
    }

    pub(crate) fn to_rgb(&self, c: f32) -> [u8; 3] {
        let g = self.0.gamma;
        let (_xw, yw, _zw) = {
            let wp = self.0.white_point;
            (wp[0], wp[1], wp[2])
        };
        let (_xb, _yb, _zb) = {
            let bp = self.0.black_point;
            (bp[0], bp[1], bp[2])
        };

        let a = c;
        let ag = a.powf(g);
        let l = yw * ag;
        let val = (0.0f32.max(295.8 * l.powf(0.3333333333333333) - 40.8) + 0.5) as u8;

        [val, val, val]
    }
}

#[derive(Debug)]
struct CalRgbRepr {
    white_point: [f32; 3],
    black_point: [f32; 3],
    matrix: [f32; 9],
    gamma: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct CalRgb(Arc<CalRgbRepr>);

// See <https://github.com/mozilla/pdf.js/blob/06f44916c8936b92f464d337fe3a0a6b2b78d5b4/src/core/colorspace.js#L846>
// Completely copied from there without really understanding the logic, but we get the same results as Firefox
// which should be good enough (and by viewing the `calrgb.pdf` test file in different viewers you will
// see that in many cases each viewer does whatever it wants, even Acrobat), so this is good enough for us.
impl CalRgb {
    pub fn new(dict: &Dict) -> Option<Self> {
        let white_point = dict.get::<[f32; 3]>(WHITE_POINT).unwrap_or([1.0, 1.0, 1.0]);
        let black_point = dict.get::<[f32; 3]>(BLACK_POINT).unwrap_or([0.0, 0.0, 0.0]);
        let matrix = dict
            .get::<[f32; 9]>(MATRIX)
            .unwrap_or([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let gamma = dict.get::<[f32; 3]>(GAMMA).unwrap_or([1.0, 1.0, 1.0]);

        Some(Self(Arc::new(CalRgbRepr {
            white_point,
            black_point,
            matrix,
            gamma,
        })))
    }

    const BRADFORD_SCALE_MATRIX: [f32; 9] = [
        0.8951, 0.2664, -0.1614, -0.7502, 1.7135, 0.0367, 0.0389, -0.0685, 1.0296,
    ];

    const BRADFORD_SCALE_INVERSE_MATRIX: [f32; 9] = [
        0.9869929, -0.1470543, 0.1599627, 0.4323053, 0.5183603, 0.0492912, -0.0085287, 0.0400428,
        0.9684867,
    ];

    const SRGB_D65_XYZ_TO_RGB_MATRIX: [f32; 9] = [
        3.2404542, -1.5371385, -0.4985314, -0.9692660, 1.8760108, 0.0415560, 0.0556434, -0.2040259,
        1.0572252,
    ];

    const FLAT_WHITEPOINT: [f32; 3] = [1.0, 1.0, 1.0];
    const D65_WHITEPOINT: [f32; 3] = [0.95047, 1.0, 1.08883];

    fn decode_l_constant() -> f32 {
        ((8.0f32 + 16.0) / 116.0).powi(3) / 8.0
    }

    fn srgb_transfer_function(color: f32) -> f32 {
        if color <= 0.0031308 {
            (12.92 * color).clamp(0.0, 1.0)
        } else if color >= 0.99554525 {
            1.0
        } else {
            ((1.0 + 0.055) * color.powf(1.0 / 2.4) - 0.055).clamp(0.0, 1.0)
        }
    }

    fn matrix_product(a: &[f32; 9], b: &[f32; 3]) -> [f32; 3] {
        [
            a[0] * b[0] + a[1] * b[1] + a[2] * b[2],
            a[3] * b[0] + a[4] * b[1] + a[5] * b[2],
            a[6] * b[0] + a[7] * b[1] + a[8] * b[2],
        ]
    }

    fn to_flat(source_white_point: &[f32; 3], lms: &[f32; 3]) -> [f32; 3] {
        [
            lms[0] / source_white_point[0],
            lms[1] / source_white_point[1],
            lms[2] / source_white_point[2],
        ]
    }

    fn to_d65(source_white_point: &[f32; 3], lms: &[f32; 3]) -> [f32; 3] {
        [
            lms[0] * Self::D65_WHITEPOINT[0] / source_white_point[0],
            lms[1] * Self::D65_WHITEPOINT[1] / source_white_point[1],
            lms[2] * Self::D65_WHITEPOINT[2] / source_white_point[2],
        ]
    }

    fn decode_l(l: f32) -> f32 {
        if l < 0.0 {
            -Self::decode_l(-l)
        } else if l > 8.0 {
            ((l + 16.0) / 116.0).powi(3)
        } else {
            l * Self::decode_l_constant()
        }
    }

    fn compensate_black_point(source_bp: &[f32; 3], xyz_flat: &[f32; 3]) -> [f32; 3] {
        if source_bp == &[0.0, 0.0, 0.0] {
            return *xyz_flat;
        }

        let zero_decode_l = Self::decode_l(0.0);

        let mut out = [0.0; 3];
        for i in 0..3 {
            let src = Self::decode_l(source_bp[i]);
            let scale = (1.0 - zero_decode_l) / (1.0 - src);
            let offset = 1.0 - scale;
            out[i] = xyz_flat[i] * scale + offset;
        }

        out
    }

    fn normalize_white_point_to_flat(
        &self,
        source_white_point: &[f32; 3],
        xyz: &[f32; 3],
    ) -> [f32; 3] {
        if source_white_point[0] == 1.0 && source_white_point[2] == 1.0 {
            return *xyz;
        }
        let lms = Self::matrix_product(&Self::BRADFORD_SCALE_MATRIX, xyz);
        let lms_flat = Self::to_flat(source_white_point, &lms);
        Self::matrix_product(&Self::BRADFORD_SCALE_INVERSE_MATRIX, &lms_flat)
    }

    fn normalize_white_point_to_d65(
        &self,
        source_white_point: &[f32; 3],
        xyz: &[f32; 3],
    ) -> [f32; 3] {
        let lms = Self::matrix_product(&Self::BRADFORD_SCALE_MATRIX, xyz);
        let lms_d65 = Self::to_d65(source_white_point, &lms);
        Self::matrix_product(&Self::BRADFORD_SCALE_INVERSE_MATRIX, &lms_d65)
    }

    pub(crate) fn to_rgb(&self, mut c: [f32; 3]) -> [u8; 3] {
        for i in &mut c {
            *i = i.clamp(0.0, 1.0);
        }

        let [r, g, b] = c;
        let [gr, gg, gb] = self.0.gamma;
        let [agr, bgg, cgb] = [
            if r == 1.0 { 1.0 } else { r.powf(gr) },
            if g == 1.0 { 1.0 } else { g.powf(gg) },
            if b == 1.0 { 1.0 } else { b.powf(gb) },
        ];

        let m = &self.0.matrix;
        let x = m[0] * agr + m[3] * bgg + m[6] * cgb;
        let y = m[1] * agr + m[4] * bgg + m[7] * cgb;
        let z = m[2] * agr + m[5] * bgg + m[8] * cgb;
        let xyz = [x, y, z];

        let xyz_flat = self.normalize_white_point_to_flat(&self.0.white_point, &xyz);
        let xyz_black = Self::compensate_black_point(&self.0.black_point, &xyz_flat);
        let xyz_d65 = self.normalize_white_point_to_d65(&Self::FLAT_WHITEPOINT, &xyz_black);
        let srgb_xyz = Self::matrix_product(&Self::SRGB_D65_XYZ_TO_RGB_MATRIX, &xyz_d65);

        [
            (Self::srgb_transfer_function(srgb_xyz[0]) * 255.0 + 0.5) as u8,
            (Self::srgb_transfer_function(srgb_xyz[1]) * 255.0 + 0.5) as u8,
            (Self::srgb_transfer_function(srgb_xyz[2]) * 255.0 + 0.5) as u8,
        ]
    }
}

#[derive(Debug)]
struct LabRepr {
    white_point: [f32; 3],
    black_point: [f32; 3],
    range: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct Lab(Arc<LabRepr>);

impl Lab {
    pub fn new(dict: &Dict) -> Option<Self> {
        let white_point = dict.get::<[f32; 3]>(WHITE_POINT).unwrap_or([1.0, 1.0, 1.0]);
        let black_point = dict.get::<[f32; 3]>(BLACK_POINT).unwrap_or([0.0, 0.0, 0.0]);
        let range = dict
            .get::<[f32; 4]>(RANGE)
            .unwrap_or([-100.0, 100.0, -100.0, 100.0]);

        Some(Self(Arc::new(LabRepr {
            white_point,
            black_point,
            range,
        })))
    }
    fn decode(value: f32, high1: f32, low2: f32, high2: f32) -> f32 {
        low2 + (value * (high2 - low2)) / high1
    }

    fn fn_g(x: f32) -> f32 {
        if x >= 6.0 / 29.0 {
            x.powi(3)
        } else {
            (108.0 / 841.0) * (x - 4.0 / 29.0)
        }
    }

    pub(crate) fn to_rgb(&self, c: [f32; 3]) -> [u8; 3] {
        let LabRepr { white_point, .. } = &*self.0;

        let (l, a, b) = (c[0], c[1], c[2]);

        let m = (l + 16.0) / 116.0;
        let l = m + a / 500.0;
        let n = m - b / 200.0;

        let x = white_point[0] * Self::fn_g(l);
        let y = white_point[1] * Self::fn_g(m);
        let z = white_point[2] * Self::fn_g(n);

        let (r, g, b) = if white_point[2] < 1.0 {
            (
                x * 3.1339 + y * -1.617 + z * -0.4906,
                x * -0.9785 + y * 1.916 + z * 0.0333,
                x * 0.072 + y * -0.229 + z * 1.4057,
            )
        } else {
            (
                x * 3.2406 + y * -1.5372 + z * -0.4986,
                x * -0.9689 + y * 1.8758 + z * 0.0415,
                x * 0.0557 + y * -0.204 + z * 1.057,
            )
        };

        let conv = |v: f32| (v.max(0.0).sqrt() * 255.0).clamp(0.0, 255.0) as u8;

        [conv(r), conv(g), conv(b)]
    }
}

#[derive(Debug)]
struct IndexedRepr {
    values: Vec<Vec<f32>>,
    hival: u8,
    base: Box<ColorSpace>,
}

#[derive(Debug, Clone)]
pub struct Indexed(Arc<IndexedRepr>);

impl Indexed {
    pub fn new(array: &Array) -> Option<Self> {
        let mut iter = array.iter::<Object>();
        // Skip name
        let _ = iter.next()?;
        let base_color_space = ColorSpace::new(iter.next()?);
        let hival = iter.next()?.cast::<u8>().ok()?;

        let values = {
            let next = iter.next()?;

            let data = next
                .clone()
                .cast::<Stream>()
                .ok()
                .and_then(|s| s.decoded().ok())
                .or_else(|| next.clone().cast::<string::String>().ok().map(|s| s.get()))
                .unwrap();

            let num_components = base_color_space.components();

            let mut byte_iter = data.iter().copied();

            let mut vals = vec![];
            for _ in 0..=hival {
                let mut temp = vec![];

                for _ in 0..num_components {
                    // TODO: That's probably not the proper way to scale
                    temp.push(byte_iter.next()? as f32 / 255.0)
                }

                vals.push(temp);
            }

            vals
        };

        Some(Self(Arc::new(IndexedRepr {
            values,
            hival,
            base: Box::new(base_color_space),
        })))
    }

    pub fn to_rgb(&self, val: f32, opacity: f32) -> AlphaColor<Srgb> {
        let idx = (val.clamp(0.0, self.0.hival as f32) + 0.5) as usize;
        self.0.base.to_rgba(self.0.values[idx].as_slice(), opacity)
    }
}

#[derive(Clone, Debug)]
pub enum ColorType {
    DeviceRgb([f32; 3]),
    DeviceGray(f32),
    DeviceCmyk([f32; 4]),
    Icc(ICCProfile, ColorComponents),
    CalGray(CalGray, f32),
    CalRgb(CalRgb, [f32; 3]),
    Lab(Lab, [f32; 3]),
    Indexed(Indexed, f32),
}

struct ICCColorRepr {
    transform: Transform,
    number_components: usize,
}

#[derive(Clone)]
pub struct ICCProfile(Arc<ICCColorRepr>);

impl Debug for ICCProfile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ICCColor {{..}}")
    }
}

impl ICCProfile {
    pub fn new(profile: &[u8], number_components: usize) -> Option<Self> {
        let input = qcms::Profile::new_from_slice(profile, false)?;
        let mut output = qcms::Profile::new_sRGB();
        output.precache_output_transform();

        let data_type = match number_components {
            1 => qcms::DataType::Gray8,
            3 => qcms::DataType::RGB8,
            4 => qcms::DataType::CMYK,
            _ => {
                warn!(
                    "unsupported number of components {} for ICC profile",
                    number_components
                );

                return None;
            }
        };

        let transform = Transform::new_to(
            &input,
            &output,
            data_type,
            qcms::DataType::RGB8,
            qcms::Intent::default(),
        )?;

        Some(Self(Arc::new(ICCColorRepr {
            transform,
            number_components,
        })))
    }

    pub(crate) fn to_rgba(&self, c: &[f32]) -> [u8; 3] {
        let mut srgb = [0, 0, 0];

        match self.0.number_components {
            1 => self.0.transform.convert(&[u8_to_f32(c[0])], &mut srgb),
            3 => self.0.transform.convert(
                &[u8_to_f32(c[0]), u8_to_f32(c[1]), u8_to_f32(c[2])],
                &mut srgb,
            ),
            4 => self.0.transform.convert(
                &[
                    u8_to_f32(c[0]),
                    u8_to_f32(c[1]),
                    u8_to_f32(c[2]),
                    u8_to_f32(c[3]),
                ],
                &mut srgb,
            ),
            _ => unreachable!(),
        }

        srgb
    }
}

fn u8_to_f32(val: f32) -> u8 {
    (val * 255.0 + 0.5) as u8
}

#[derive(Clone, Debug)]
pub struct Color {
    color_type: ColorType,
    opacity: f32,
}

impl Color {
    pub(crate) fn from_pdf(color_space: ColorSpace, c: &ColorComponents, opacity: f32) -> Self {
        let c_type = match color_space {
            ColorSpace::DeviceCmyk => ColorType::DeviceCmyk([c[0], c[1], c[2], c[3]]),
            ColorSpace::DeviceGray => ColorType::DeviceGray(c[0]),
            ColorSpace::DeviceRgb => ColorType::DeviceRgb([c[0], c[1], c[2]]),
            ColorSpace::ICCColor(icc) => ColorType::Icc(icc, c.clone()),
            ColorSpace::CalGray(cal) => ColorType::CalGray(cal, c[0]),
            ColorSpace::CalRgb(cal) => ColorType::CalRgb(cal, [c[0], c[1], c[2]]),
            ColorSpace::Lab(lab) => ColorType::Lab(lab, [c[0], c[1], c[2]]),
            ColorSpace::Indexed(i) => ColorType::Indexed(i, c[0]),
        };

        Self {
            color_type: c_type,
            opacity,
        }
    }

    pub fn to_rgba(&self) -> AlphaColor<Srgb> {
        match &self.color_type {
            // TODO: Deduplicate
            ColorType::DeviceRgb(r) => AlphaColor::new([r[0], r[1], r[2], self.opacity]),
            ColorType::DeviceGray(g) => AlphaColor::new([*g, *g, *g, self.opacity]),
            ColorType::DeviceCmyk(c) => {
                let opacity = u8_to_f32(self.opacity);
                let srgb = CMYK_TRANSFORM.to_rgba(&c[..]);

                let res = AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity);

                res
            }
            ColorType::Icc(icc, c) => {
                let opacity = u8_to_f32(self.opacity);
                let srgb = icc.to_rgba(&c[..]);

                let res = AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity);

                res
            }
            ColorType::CalGray(cal, c) => {
                let opacity = u8_to_f32(self.opacity);
                let srgb = cal.to_rgb(*c);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorType::CalRgb(cal, c) => {
                let opacity = u8_to_f32(self.opacity);
                let srgb = cal.to_rgb(*c);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorType::Lab(lab, c) => {
                let opacity = u8_to_f32(self.opacity);
                let srgb = lab.to_rgb(*c);

                AlphaColor::from_rgba8(srgb[0], srgb[1], srgb[2], opacity)
            }
            ColorType::Indexed(i, c) => i.to_rgb(*c, self.opacity),
        }
    }
}

static CMYK_TRANSFORM: Lazy<ICCProfile> = Lazy::new(|| {
    ICCProfile::new(
        include_bytes!("../../assets/CGATS001Compat-v2-micro.icc"),
        4,
    )
    .unwrap()
});
