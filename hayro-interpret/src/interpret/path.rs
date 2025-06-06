use crate::Paint;
use crate::color::Color;
use crate::context::Context;
use crate::device::Device;
use crate::glyph::Glyph;
use crate::paint::PaintType;
use kurbo::{Affine, BezPath};
use log::warn;

pub(crate) fn fill_path(context: &mut Context, device: &mut impl Device) {
    fill_path_impl(context, device, None, None);

    context.path_mut().truncate(0);
}

pub(crate) fn stroke_path(context: &mut Context, device: &mut impl Device) {
    stroke_path_impl(context, device, None, None);

    context.path_mut().truncate(0);
}

pub(crate) fn fill_stroke_path(context: &mut Context, device: &mut impl Device) {
    fill_path_impl(context, device, None, None);
    stroke_path_impl(context, device, None, None);

    context.path_mut().truncate(0);
}

pub(crate) fn fill_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    // TODO: DOn't take option here?
    path: Option<&BezPath>,
    transform: Option<Affine>,
) {
    let base_transform = transform.unwrap_or(context.get().ctm);
    device.set_transform(base_transform);

    let paint = get_paint(context, false);
    device.set_fill_properties(&context.fill_props());

    match path {
        None => device.fill_path(context.path(), &paint),
        Some(path) => device.fill_path(path, &paint),
    };
}

pub(crate) fn clip_impl(context: &mut Context, glyph: &Glyph, transform: Affine) {
    match glyph {
        Glyph::Outline(o) => {
            let outline = transform * o.outline();
            let has_outline = outline.segments().next().is_some();

            if has_outline {
                context.get_mut().text_state.clip_paths.extend(outline);
            }
        }
        Glyph::Shape(_) => {
            warn!("text rendering mode clip not implemented for shape glyphs");
        }
    }
}

pub(crate) fn stroke_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    path: Option<&BezPath>,
    transform: Option<Affine>,
) {
    let base_transform = transform.unwrap_or(context.get().ctm);
    device.set_transform(base_transform);

    device.set_stroke_properties(&context.stroke_props());
    let paint = get_paint(context, true);

    match path {
        None => device.stroke_path(context.path(), &paint),
        Some(path) => device.stroke_path(path, &paint),
    };
}

pub(crate) fn get_paint<'a>(context: &Context<'a>, is_stroke: bool) -> Paint<'a> {
    let data = if is_stroke {
        context.get().stroke_data()
    } else {
        context.get().non_stroke_data()
    };

    // TODO: use let chains
    if data.color_space.is_pattern() && data.pattern.is_some() {
        let pattern = data.pattern.unwrap().clone();

        Paint {
            paint_type: PaintType::Pattern(pattern),
            paint_transform: context.root_transform(),
        }
    } else {
        let color = Color::new(data.color_space, data.color, data.alpha);

        Paint {
            paint_type: PaintType::Color(color),
            paint_transform: Affine::IDENTITY,
        }
    }
}
