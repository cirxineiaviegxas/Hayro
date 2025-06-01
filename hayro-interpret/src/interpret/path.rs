use crate::color::Color;
use crate::context::Context;
use crate::device::Device;
use crate::font::GlyphDescription;
use crate::Paint;
use kurbo::Affine;
use log::warn;
use crate::interpret::text::run_t3_instructions;

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

// TODO: Get rid of glyph description

pub(crate) fn fill_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    path: Option<&GlyphDescription>,
    transform: Option<Affine>,
) {
    let base_transform = transform.unwrap_or(context.get().ctm);
    device.set_transform(base_transform);

    set_device_paint(context, device, false);

    match path {
        None => device.fill_path(context.path(), &context.fill_props()),
        Some(GlyphDescription::Path(path)) => device.fill_path(path, &context.fill_props()),
        Some(GlyphDescription::Type3(t3)) => run_t3_instructions(device, t3, base_transform * t3.1),
    };
}

pub(crate) fn clip_impl(context: &mut Context, outline: &GlyphDescription) {
    match outline {
        GlyphDescription::Path(p) => {
            let has_outline = p.segments().next().is_some();

            if has_outline {
                context.get_mut().text_state.clip_paths.extend(p);
            }
        }
        GlyphDescription::Type3(_) => {
            warn!("text rendering mode clip is currently not supported with Type3 glyphs")
        }
    }
}

pub(crate) fn stroke_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    path: Option<&GlyphDescription>,
    transform: Option<Affine>,
) {
    let base_transform = transform.unwrap_or(context.get().ctm);
    device.set_transform(base_transform);

    set_device_paint(context, device, true);

    match path {
        None => device.stroke_path(context.path(), &context.stroke_props()),
        Some(GlyphDescription::Path(path)) => device.stroke_path(path, &context.stroke_props()),
        Some(GlyphDescription::Type3(t3)) => run_t3_instructions(device, t3, base_transform * t3.1),
    };
}

pub(crate) fn set_device_paint(context: &mut Context, device: &mut impl Device, is_stroke: bool) {
    let data = if is_stroke {
        context.get().stroke_data()
    } else {
        context.get().non_stroke_data()
    };

    // TODO: use let chains
    if data.color_space.is_pattern() && data.pattern.is_some() {
        let pattern = data.pattern.unwrap();
        device.set_paint_transform(context.root_transform());
        device.set_paint(Paint::Shading(pattern));
    } else {
        let color = Color::new(data.color_space, data.color, data.alpha);
        device.set_paint(Paint::Color(color));
    };
}
