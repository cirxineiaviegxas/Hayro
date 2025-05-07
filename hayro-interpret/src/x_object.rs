use crate::context::Context;
use crate::device::Device;
use crate::interpret;
use hayro_syntax::content::ops::XObject;
use hayro_syntax::content::{TypedIter, UntypedIter};
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BBOX, FONT_MATRIX, GROUP, REF, RESOURCES, SUBTYPE};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, Rect, Shape};
use log::warn;
use peniko::Fill;
use std::borrow::Cow;

pub struct FormXObject<'a> {
    decoded: Cow<'a, [u8]>,
    matrix: Affine,
    bbox: [f32; 4],
    resources: Dict<'a>,
}

impl<'a> FormXObject<'a> {
    pub fn new(stream: &Stream<'a>) -> Self {
        let dict = stream.dict();

        if dict.get::<Name>(SUBTYPE).unwrap().as_str() != "Form" {
            panic!("only form x object are currently supported.")
        }
        
        if dict.contains_key(REF) {
            warn!("reference xobjects are not supported.");
        }

        let decoded = stream.decoded().unwrap();
        let resources = dict.get::<Dict>(RESOURCES).unwrap_or_default();

        let matrix = Affine::new(
            dict.get::<[f64; 6]>(FONT_MATRIX)
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
    x_object: &FormXObject<'a>,
    context: &mut Context<'a>,
    device: &mut impl Device,
) {
    let iter = TypedIter::new(UntypedIter::new(x_object.decoded.as_ref()));

    context.save_state();
    context.pre_concat_affine(x_object.matrix);
    device.set_transform(context.get().affine);
    device.push_clip(
        &Rect::new(
            x_object.bbox[0] as f64,
            x_object.bbox[1] as f64,
            x_object.bbox[2] as f64,
            x_object.bbox[3] as f64,
        )
        .to_path(0.1),
        Fill::NonZero,
    );
    interpret(iter, &x_object.resources, context, device);
    device.pop_clip();
    context.restore_state();
}
