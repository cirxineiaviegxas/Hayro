use crate::color::{Color, ColorSpace};
use crate::context::Context;
use crate::device::{ClipPath, Device};
use crate::interpret;
use bitreader::BitReader;
use hayro_syntax::content::{TypedIter, UntypedIter};
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{
    BBOX, BITS_PER_COMPONENT, BPC, COLORSPACE, CS, D, DECODE, H, HEIGHT, I, IM, IMAGE_MASK,
    INTERPOLATE, MATRIX, RESOURCES, SMASK, SUBTYPE, W, WIDTH,
};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, Rect, Shape};
use peniko::{Fill, ImageQuality};
use std::borrow::Cow;

pub enum XObject<'a> {
    FormXObject(FormXObject<'a>),
    ImageXObject(ImageXObject<'a>),
}

impl<'a> XObject<'a> {
    pub fn new(stream: &Stream<'a>) -> Option<Self> {
        let dict = stream.dict();
        match dict.get::<Name>(SUBTYPE).unwrap().as_ref() {
            b"Image" => Some(Self::ImageXObject(ImageXObject::new(stream)?)),
            b"Form" => Some(Self::FormXObject(FormXObject::new(stream))),
            _ => unimplemented!(),
        }
    }
}

pub struct FormXObject<'a> {
    pub decoded: Cow<'a, [u8]>,
    matrix: Affine,
    bbox: [f32; 4],
    resources: Dict<'a>,
}

impl<'a> FormXObject<'a> {
    fn new(stream: &Stream<'a>) -> Self {
        let dict = stream.dict();

        let decoded = stream.decoded().unwrap();
        let resources = dict.get::<Dict>(RESOURCES).unwrap_or_default();

        let matrix = Affine::new(
            dict.get::<[f64; 6]>(MATRIX)
                .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        );
        let bbox = dict.get::<[f32; 4]>(BBOX).unwrap();

        Self {
            decoded,
            matrix,
            bbox,
            resources,
        }
    }
}

pub(crate) fn draw_xobject<'a>(
    x_object: &XObject<'a>,
    context: &mut Context<'a>,
    device: &mut impl Device,
) {
    match x_object {
        XObject::FormXObject(f) => draw_form_xobject(f, context, device),
        XObject::ImageXObject(i) => {
            draw_image_xobject(i, context, device);
        }
    }
}

pub(crate) fn draw_form_xobject<'a>(
    x_object: &FormXObject<'a>,
    context: &mut Context<'a>,
    device: &mut impl Device,
) {
    let iter = TypedIter::new(UntypedIter::new(x_object.decoded.as_ref()));

    context.save_state();
    context.pre_concat_affine(x_object.matrix);
    device.set_transform(context.get().affine);
    device.push_layer(
        Some(&ClipPath {
            path: Rect::new(
                x_object.bbox[0] as f64,
                x_object.bbox[1] as f64,
                x_object.bbox[2] as f64,
                x_object.bbox[3] as f64,
            )
            .to_path(0.1),
            fill: Fill::NonZero,
        }),
        context.get().fill_alpha,
    );
    interpret(iter, &x_object.resources, context, device);
    device.pop();
    context.restore_state();
}

pub(crate) fn draw_image_xobject(
    x_object: &ImageXObject<'_>,
    context: &mut Context<'_>,
    device: &mut impl Device,
) {
    let width = x_object.width as f64;
    let height = x_object.height as f64;

    let color = Color::from_pdf(
        context.get().fill_cs.clone(),
        &context.get().fill_color,
        context.get().fill_alpha,
    );

    let data = x_object.as_rgba8(color);

    // TODO: image_ccit test cases look pretty bad, we need support for mipmaps to improve
    // them.
    let quality = if x_object.interpolate || x_object.bits_per_component <= 8 {
        ImageQuality::Medium
    } else {
        ImageQuality::Low
    };

    context.save_state();
    context.pre_concat_affine(Affine::new([
        1.0 / width,
        0.0,
        0.0,
        -1.0 / height,
        0.0,
        1.0,
    ]));
    device.set_transform(context.get().affine);
    device.draw_rgba_image(
        data,
        x_object.width,
        x_object.height,
        x_object.is_mask,
        quality,
    );
    context.restore_state();
}

pub struct ImageXObject<'a> {
    pub decoded: Cow<'a, [u8]>,
    pub width: u32,
    pub height: u32,
    color_space: ColorSpace,
    interpolate: bool,
    decode: Vec<(f32, f32)>,
    is_mask: bool,
    pub dict: Dict<'a>,
    bits_per_component: u8,
}

impl<'a> ImageXObject<'a> {
    pub(crate) fn new(stream: &Stream<'a>) -> Option<Self> {
        let dict = stream.dict();

        let decoded = stream.decoded().unwrap();
        let interpolate = dict
            .get::<bool>(INTERPOLATE)
            .or_else(|| dict.get::<bool>(I))
            .unwrap_or(false);
        let image_mask = dict
            .get::<bool>(IMAGE_MASK)
            .or_else(|| dict.get::<bool>(IM))
            .unwrap_or(false);
        let bits_per_component = if image_mask {
            1
        } else {
            dict.get::<u8>(BITS_PER_COMPONENT)
                .or_else(|| dict.get::<u8>(BPC))
                .unwrap()
        };
        let color_space = if image_mask {
            ColorSpace::DeviceGray
        } else {
            ColorSpace::new(
                dict.get::<Object>(COLORSPACE)
                    .or_else(|| dict.get::<Object>(CS))
                    .unwrap(),
            )
        };
        let decode = dict
            .get::<Array>(DECODE)
            .or_else(|| dict.get::<Array>(D))
            .map(|a| {
                let vals = a.iter::<f32>().collect::<Vec<_>>();
                vals.chunks(2).map(|v| (v[0], v[1])).collect::<Vec<_>>()
            })
            .unwrap_or(color_space.default_decode_arr());
        let width = dict
            .get::<u32>(WIDTH)
            .or_else(|| dict.get::<u32>(W))
            .unwrap();
        let height = dict
            .get::<u32>(HEIGHT)
            .or_else(|| dict.get::<u32>(H))
            .unwrap();

        Some(Self {
            decoded,
            width,
            height,
            color_space,
            interpolate,
            decode,
            is_mask: image_mask,
            dict: dict.clone(),
            bits_per_component,
        })
    }

    pub fn as_rgba8(&self, current_color: Color) -> Vec<u8> {
        if self.is_mask {
            let decoded = self.decode_raw();
            decoded
                .iter()
                .flat_map(|alpha| {
                    current_color
                        .to_rgba()
                        .multiply_alpha(1.0 - *alpha)
                        .to_rgba8()
                        .to_u8_array()
                })
                .collect()
        } else {
            let s_mask = self
                .dict
                .get::<Stream>(SMASK)
                .and_then(|s| ImageXObject::new(&s).map(|s| s.decode_raw()))
                .unwrap_or(vec![1.0; self.width as usize * self.height as usize]);

            self.decode_raw()
                .chunks(self.color_space.components() as usize)
                .zip(s_mask)
                .flat_map(|(v, alpha)| self.color_space.to_rgba(v, alpha).to_rgba8().to_u8_array())
                .collect::<Vec<_>>()
        }
    }

    pub fn decode_raw(&self) -> Vec<f32> {
        let interpolate =
            |n: f32, d_min: f32, d_max: f32| d_min + (n * (d_max - d_min) / (2.0f32.powi(8) - 1.0));

        let mut adjusted_components = match self.bits_per_component {
            1 | 2 | 4 => {
                let mut buf = vec![];
                let mut reader = BitReader::new(self.decoded.as_ref());
                
                for _ in 0..self.height {
                    for _ in 0..self.width {
                        
                        // See `stream_ccit_not_enough_data`, some images seemingly don't have
                        // enough data, so we just pad with zeroes in this case.
                        let next = reader.read_u8(self.bits_per_component).unwrap_or(0);
                        let mapped = next as u16 * 255 / ((1 << self.bits_per_component) - 1);

                        buf.push(mapped as u8);
                    }

                    reader.align(1).unwrap();
                }

                buf
            }
            8 => self.decoded.to_vec(),
            16 => self
                .decoded
                .chunks(2)
                .map(|v| (u16::from_be_bytes([v[0], v[1]]) >> 8) as u8)
                .collect(),
            _ => unimplemented!(),
        };

        let mut decoded_arr = vec![];

        for components in adjusted_components.chunks(self.color_space.components() as usize) {
            for (component, (d_min, d_max)) in components.iter().zip(&self.decode) {
                decoded_arr.push(interpolate(*component as f32, *d_min, *d_max));
            }
        }

        decoded_arr
    }
}
