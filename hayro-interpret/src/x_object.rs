use crate::context::Context;
use crate::device::Device;
use crate::interpret;
use hayro_syntax::content::{TypedIter, UntypedIter};
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BBOX, FONT_MATRIX, GROUP, RESOURCES, SUBTYPE};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, Rect, Shape};
use log::warn;
use peniko::Fill;

pub(crate) fn draw_xobject<'a>(
    stream: &Stream<'a>,
    context: &mut Context<'a>,
    device: &mut impl Device,
) {
    let dict = stream.dict();

    if dict.get::<Name>(SUBTYPE).unwrap().as_str() != "Form" {
        panic!("only form x object are currently supported.")
    }

    if dict.contains_key(GROUP) {
        warn!("transparency groups are currently not supported.")
    }

    let stream_data = stream.decoded().unwrap();
    let iter = TypedIter::new(UntypedIter::new(stream_data.as_ref()));
    let resources = dict.get::<Dict>(RESOURCES).unwrap_or_default();

    let matrix = Affine::new(
        dict.get::<[f64; 6]>(FONT_MATRIX)
            .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
    );
    let bbox = dict.get::<[f32; 4]>(BBOX).unwrap();

    context.save_state();
    context.pre_concat_affine(matrix);
    device.set_transform(context.get().affine);
    device.push_clip(
        &Rect::new(
            bbox[0] as f64,
            bbox[1] as f64,
            bbox[2] as f64,
            bbox[3] as f64,
        )
        .to_path(0.1),
        Fill::NonZero,
    );
    interpret(iter, resources, context, device);
    device.pop_clip();
    context.restore_state();
}
