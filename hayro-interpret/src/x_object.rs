use crate::color::ColorSpace;
use crate::context::Context;
use crate::device::Device;
use crate::{handle_paint, interpret, FillProps};
use hayro_syntax::bit::{BitReader, BitSize};
use hayro_syntax::content::{TypedIter, UntypedIter};
use hayro_syntax::document::page::Resources;
use hayro_syntax::function::interpolate;
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::*;
use hayro_syntax::object::name::Name;
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, Rect, Shape};
use peniko::Fill;
use smallvec::SmallVec;
use crate::clip_path::ClipPath;

pub enum XObject<'a> {
    FormXObject(FormXObject<'a>),
    ImageXObject(ImageXObject<'a>),
}

impl<'a> XObject<'a> {
    pub fn new(stream: &Stream<'a>) -> Option<Self> {
        let dict = stream.dict();
        match dict.get::<Name>(SUBTYPE).unwrap() {
            IMAGE => Some(Self::ImageXObject(ImageXObject::new(stream)?)),
            FORM => Some(Self::FormXObject(FormXObject::new(stream)?)),
            _ => unimplemented!(),
        }
    }
}

pub struct FormXObject<'a> {
    pub decoded: Vec<u8>,
    matrix: Affine,
    bbox: [f32; 4],
    resources: Dict<'a>,
}

impl<'a> FormXObject<'a> {
    fn new(stream: &Stream<'a>) -> Option<Self> {
        let dict = stream.dict();

        let decoded = stream.decoded()?;
        let resources = dict.get::<Dict>(RESOURCES).unwrap_or_default();

        let matrix = Affine::new(
            dict.get::<[f64; 6]>(MATRIX)
                .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        );
        let bbox = dict.get::<[f32; 4]>(BBOX).unwrap();

        Some(Self {
            decoded,
            matrix,
            bbox,
            resources,
        })
    }
}

pub(crate) fn draw_xobject<'a>(
    x_object: &XObject<'a>,
    resources: &Resources<'a>,
    context: &mut Context<'a>,
    device: &mut impl Device,
) {
    match x_object {
        XObject::FormXObject(f) => draw_form_xobject(resources, f, context, device),
        XObject::ImageXObject(i) => {
            draw_image_xobject(i, context, device);
        }
    }
}

pub(crate) fn draw_form_xobject<'a>(
    resources: &Resources<'a>,
    x_object: &FormXObject<'a>,
    context: &mut Context<'a>,
    device: &mut impl Device,
) {
    let iter = TypedIter::new(UntypedIter::new(x_object.decoded.as_ref()));

    context.save_state();
    context.pre_concat_affine(x_object.matrix);
    context.push_root_transform();

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
    // TODO: XObjects inherit from page resources?
    interpret(
        iter,
        &Resources::from_parent(x_object.resources.clone(), resources.clone()),
        context,
        device,
    );
    device.pop();
    context.pop_root_transform();
    context.restore_state();
}

pub(crate) fn draw_image_xobject(
    x_object: &ImageXObject<'_>,
    context: &mut Context<'_>,
    device: &mut impl Device,
) {
    let width = x_object.width as f64;
    let height = x_object.height as f64;

    let data = x_object.as_rgba8();

    context.save_state();
    context.pre_concat_affine(Affine::new([
        1.0 / width,
        0.0,
        0.0,
        -1.0 / height,
        0.0,
        1.0,
    ]));
    let transform = context.get().affine;
    device.set_transform(transform);
    
    if x_object.is_image_mask {
        handle_paint(context, device, transform, false);
        device.set_anti_aliasing(false);
        
        device.push_layer(None, 1.0);
        device.fill_path(&Rect::new(0.0, 0.0, width, height).to_path(0.1), &FillProps { fill_rule: Fill::NonZero });
        device.draw_rgba_image(
            data,
            x_object.width,
            x_object.height,
            x_object.is_image_mask,
            x_object.interpolate,
        );
        device.pop();
        
        device.set_anti_aliasing(true);
    }   else {
        device.draw_rgba_image(
            data,
            x_object.width,
            x_object.height,
            x_object.is_image_mask,
            x_object.interpolate,
        );
    }
    
    context.restore_state();
}

pub struct ImageXObject<'a> {
    pub decoded: Vec<u8>,
    pub width: u32,
    pub height: u32,
    color_space: ColorSpace,
    interpolate: bool,
    decode: SmallVec<[(f32, f32); 4]>,
    is_image_mask: bool,
    pub dict: Dict<'a>,
    bits_per_component: u8,
}

impl<'a> ImageXObject<'a> {
    pub(crate) fn new(stream: &Stream<'a>) -> Option<Self> {
        let dict = stream.dict();

        let decoded = stream.decoded_image()?;
        let interpolate = dict
            .get::<bool>(I)
            .or_else(|| dict.get::<bool>(INTERPOLATE))
            .unwrap_or(false);
        let image_mask = dict
            .get::<bool>(IM)
            .or_else(|| dict.get::<bool>(IMAGE_MASK))
            .unwrap_or(false);
        let bits_per_component = if image_mask {
            1
        } else {
            dict.get::<u8>(BPC)
                .or_else(|| dict.get::<u8>(BITS_PER_COMPONENT))
                .or_else(|| decoded.bits_per_component)
                .unwrap_or(8)
        };
        let color_space = if image_mask {
            ColorSpace::device_gray()
        } else {
            dict.get::<Object>(CS)
                .or_else(|| dict.get::<Object>(COLORSPACE))
                .map(|c| ColorSpace::new(c))
                .or_else(|| {
                    decoded.color_space.map(|c| match c {
                        hayro_syntax::filter::ImageColorSpace::Gray => ColorSpace::device_gray(),
                        hayro_syntax::filter::ImageColorSpace::Rgb => ColorSpace::device_rgb(),
                        hayro_syntax::filter::ImageColorSpace::Cmyk => ColorSpace::device_cmyk(),
                    })
                })
                .unwrap_or(ColorSpace::device_gray())
        };
        let decode = dict
            .get::<Array>(D)
            .or_else(|| dict.get::<Array>(DECODE))
            .map(|a| a.iter::<(f32, f32)>().collect::<SmallVec<_>>())
            .unwrap_or(color_space.default_decode_arr(bits_per_component as f32));
        let width = dict
            .get::<u32>(W)
            .or_else(|| dict.get::<u32>(WIDTH))
            .unwrap();
        let height = dict
            .get::<u32>(H)
            .or_else(|| dict.get::<u32>(HEIGHT))
            .unwrap();

        Some(Self {
            decoded: decoded.data,
            width,
            height,
            color_space,
            interpolate,
            decode,
            is_image_mask: image_mask,
            dict: dict.clone(),
            bits_per_component,
        })
    }

    pub fn as_rgba8(&self) -> Vec<u8> {
        if self.is_image_mask {
            let decoded = self.decode_raw();
            decoded
                .iter()
                .flat_map(|alpha| {
                    let alpha = ((1.0 - *alpha) * 255.0 + 0.5) as u8;
                    [0, 0, 0, alpha]
                })
                .collect()
        } else {
            let s_mask = if let Some(s_mask) = self.dict.get::<Stream>(SMASK) {
                ImageXObject::new(&s_mask).map(|s| s.decode_raw())
            } else if let Some(mask) = self.dict.get::<Stream>(MASK) {
                if let Some(obj) = ImageXObject::new(&mask) {
                    let mut mask_data = obj.decode_raw();

                    // TODO: This is a temporary hack, we should implement resized masks
                    // properly in hayro-render

                    // Mask doesn't necessarily have the same dimensions.
                    if obj.width != self.width || obj.height != self.height {
                        let x_factor = obj.width as f32 / self.width as f32;
                        let y_factor = obj.height as f32 / self.height as f32;
                        let mut output =
                            Vec::with_capacity(self.width as usize * self.height as usize);
                        for y in 0..self.height {
                            let y = (y as f32 * y_factor).floor() as u32;
                            for x in 0..self.width {
                                let x = (x as f32 * x_factor).floor() as u32;
                                let index = y * obj.width + x;
                                output.push(mask_data[index as usize]);
                            }
                        }

                        mask_data = output;
                    }

                    mask_data = mask_data.iter().map(|v| 1.0 - *v).collect();

                    Some(mask_data)
                } else {
                    None
                }
            } else {
                None
            };

            let s_mask =
                s_mask.unwrap_or_else(|| vec![1.0; self.width as usize * self.height as usize]);

            self.decode_raw()
                .chunks(self.color_space.num_components() as usize)
                .zip(s_mask)
                .flat_map(|(v, alpha)| self.color_space.to_rgba(v, alpha).to_rgba8().to_u8_array())
                .collect::<Vec<_>>()
        }
    }

    pub fn decode_raw(&self) -> Vec<f32> {
        let interpolate = |n: f32, d_min: f32, d_max: f32| {
            interpolate(
                n,
                0.0,
                2.0f32.powi(self.bits_per_component as i32) - 1.0,
                d_min,
                d_max,
            )
        };

        let adjusted_components = match self.bits_per_component {
            1 | 2 | 4 => {
                let mut buf = vec![];
                let bpc = BitSize::from_u8(self.bits_per_component).unwrap();
                let mut reader = BitReader::new(self.decoded.as_ref());

                for _ in 0..self.height {
                    for _ in 0..self.width {
                        // See `stream_ccit_not_enough_data`, some images seemingly don't have
                        // enough data, so we just pad with zeroes in this case.
                        let next = reader.read(bpc).unwrap_or(0) as u16;

                        buf.push(next);
                    }

                    reader.align();
                }

                buf
            }
            8 => self.decoded.iter().map(|v| *v as u16).collect(),
            16 => self
                .decoded
                .chunks(2)
                .map(|v| (u16::from_be_bytes([v[0], v[1]])))
                .collect(),
            _ => unimplemented!(),
        };

        let mut decoded_arr = vec![];

        for components in adjusted_components.chunks(self.color_space.num_components() as usize) {
            for (component, (d_min, d_max)) in components.iter().zip(&self.decode) {
                decoded_arr.push(interpolate(*component as f32, *d_min, *d_max));
            }
        }

        decoded_arr
    }
}
