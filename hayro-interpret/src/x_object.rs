use crate::color::ColorSpace;
use crate::context::Context;
use crate::device::Device;
use crate::interpret;
use hayro_syntax::content::{TypedIter, UntypedIter};
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{
    BBOX, BITS_PER_COMPONENT, COLORSPACE, DECODE, HEIGHT, INTERPOLATE, MATRIX, RESOURCES, SUBTYPE,
    WIDTH,
};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, Rect, Shape};
use peniko::Fill;
use std::borrow::Cow;

pub enum XObject<'a> {
    FormXObject(FormXObject<'a>),
    ImageXObject(ImageXObject<'a>),
}

impl<'a> XObject<'a> {
    pub fn new(stream: &Stream<'a>) -> Self {
        let dict = stream.dict();
        match dict.get::<Name>(SUBTYPE).unwrap().as_ref() {
            b"Image" => Self::ImageXObject(ImageXObject::new(stream)),
            b"Form" => Self::FormXObject(FormXObject::new(stream)),
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
            println!("reached")
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
        &Rect::new(
            x_object.bbox[0] as f64,
            x_object.bbox[1] as f64,
            x_object.bbox[2] as f64,
            x_object.bbox[3] as f64,
        )
        .to_path(0.1),
        Fill::NonZero,
        (context.get().fill_alpha * 255.0 + 0.5) as u8,
    );
    interpret(iter, &x_object.resources, context, device);
    device.pop();
    context.restore_state();
}

pub struct ImageXObject<'a> {
    pub decoded: Cow<'a, [u8]>,
    width: f32,
    height: f32,
    color_space: ColorSpace,
    interpolate: bool,
    decode: Vec<(f32, f32)>,
    bits_per_component: u8,
}

impl<'a> ImageXObject<'a> {
    fn new(stream: &Stream<'a>) -> Self {
        let dict = stream.dict();

        let decoded = stream.decoded().unwrap();
        let interpolate = dict.get::<bool>(INTERPOLATE).unwrap_or(false);
        let bits_per_component = dict.get::<u8>(BITS_PER_COMPONENT).unwrap();
        let decode = {
            let arr = dict.get::<Array>(DECODE).unwrap_or_default();

            let mut vals = arr.iter::<f32>().collect::<Vec<_>>();
            vals.chunks(2).map(|v| (v[0], v[1])).collect::<Vec<_>>()
        };
        let color_space = ColorSpace::new(dict.get::<Object>(COLORSPACE).unwrap());
        let width = dict.get::<f32>(WIDTH).unwrap();
        let height = dict.get::<f32>(HEIGHT).unwrap();

        Self {
            decoded,
            width,
            height,
            color_space,
            interpolate,
            decode,
            bits_per_component,
        }
    }
}
