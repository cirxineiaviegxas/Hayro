use crate::convert::{convert_line_cap, convert_line_join};
use crate::device::Device;
use hayro_syntax::content::ops::{LineCap, LineJoin, TypedOperation};
use hayro_syntax::object::Object;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{EXT_G_STATE, FONT};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::name::names::*;
use hayro_syntax::object::number::Number;
use hayro_syntax::object::string::String;
use kurbo::{Affine, BezPath, Cap, Join, Point, Rect, Shape};
use log::warn;
use once_cell::sync::Lazy;
use peniko::Fill;
use qcms::Transform;
use skrifa::GlyphId;
use smallvec::{SmallVec, smallvec};

pub mod color;
pub mod context;
mod convert;
pub mod device;
mod font;
mod state;
mod util;

use crate::color::{Color, ColorSpace};
use crate::context::Context;
use crate::font::{Font, TextRenderingMode};
use crate::util::OptionLog;

static CMYK_TRANSFORM: Lazy<Transform> = Lazy::new(|| {
    let input = qcms::Profile::new_from_slice(
        include_bytes!("../../assets/CGATS001Compat-v2-micro.icc"),
        false,
    )
    .unwrap();
    let mut output = qcms::Profile::new_sRGB();
    output.precache_output_transform();

    Transform::new_to(
        &input,
        &output,
        qcms::DataType::CMYK,
        qcms::DataType::RGB8,
        qcms::Intent::default(),
    )
    .unwrap()
});

pub struct StrokeProps {
    pub line_width: f32,
    pub line_cap: Cap,
    pub line_join: Join,
    pub miter_limit: f32,
    pub dash_array: SmallVec<[f32; 4]>,
    pub dash_offset: f32,
}

pub struct FillProps {
    pub fill_rule: Fill,
}

pub fn interpret<'a>(
    ops: impl Iterator<Item = TypedOperation<'a>>,
    resources: Dict,
    context: &mut Context,
    device: &mut impl Device,
) {
    let ext_g_states = resources.get::<Dict>(EXT_G_STATE).unwrap_or_default();
    let fonts = resources.get::<Dict>(FONT).unwrap_or_default();

    for op in ops {
        match op {
            TypedOperation::SaveState(_) => context.save_state(),
            TypedOperation::StrokeColorDeviceRgb(s) => {
                context.get_mut().stroke_cs = ColorSpace::DeviceRgb;
                context.get_mut().stroke_color =
                    smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32()];
            }
            TypedOperation::StrokeColorDeviceGray(s) => {
                context.get_mut().stroke_cs = ColorSpace::DeviceGray;
                context.get_mut().stroke_color = smallvec![s.0.as_f32()];
            }
            TypedOperation::StrokeColorCmyk(s) => {
                context.get_mut().stroke_cs = ColorSpace::DeviceCmyk;
                context.get_mut().stroke_color =
                    smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32(), s.3.as_f32()];
            }
            TypedOperation::LineWidth(w) => {
                context.get_mut().line_width = w.0.as_f32();
            }
            TypedOperation::LineCap(c) => {
                context.get_mut().line_cap = convert_line_cap(c);
            }
            TypedOperation::LineJoin(j) => {
                context.get_mut().line_join = convert_line_join(j);
            }
            TypedOperation::MiterLimit(l) => {
                context.get_mut().miter_limit = l.0.as_f32();
            }
            TypedOperation::Transform(t) => {
                context.pre_concat_transform(t);
            }
            TypedOperation::RectPath(r) => {
                let rect = Rect::new(
                    r.0.as_f64(),
                    r.1.as_f64(),
                    r.0.as_f64() + r.2.as_f64(),
                    r.1.as_f64() + r.3.as_f64(),
                )
                .to_path(0.1);
                context.path_mut().extend(rect);
            }
            TypedOperation::MoveTo(m) => {
                let p = Point::new(m.0.as_f64(), m.1.as_f64());
                *(context.last_point_mut()) = p;
                *(context.sub_path_start_mut()) = p;
                context.path_mut().move_to(p);
            }
            TypedOperation::FillPathEvenOdd(_) => {
                context.get_mut().fill = Fill::EvenOdd;
                fill_path(context, device);
            }
            TypedOperation::FillPathNonZero(_) => {
                context.get_mut().fill = Fill::NonZero;
                fill_path(context, device);
            }
            TypedOperation::FillPathNonZeroCompatibility(_) => {
                context.get_mut().fill = Fill::NonZero;
                fill_path(context, device);
            }
            TypedOperation::FillAndStrokeEvenOdd(_) => {
                context.get_mut().fill = Fill::EvenOdd;
                fill_stroke_path(context, device);
            }
            TypedOperation::FillAndStrokeNonZero(_) => {
                context.get_mut().fill = Fill::NonZero;
                fill_stroke_path(context, device);
            }
            TypedOperation::CloseAndStrokePath(_) => {
                context.path_mut().close_path();
                stroke_path(context, device);
            }
            TypedOperation::CloseFillAndStrokeEvenOdd(_) => {
                context.path_mut().close_path();
                context.get_mut().fill = Fill::EvenOdd;
                fill_stroke_path(context, device);
            }
            TypedOperation::CloseFillAndStrokeNonZero(_) => {
                context.path_mut().close_path();
                context.get_mut().fill = Fill::NonZero;
                fill_stroke_path(context, device);
            }
            TypedOperation::NonStrokeColorDeviceGray(s) => {
                context.get_mut().fill_cs = ColorSpace::DeviceGray;
                context.get_mut().fill_color = smallvec![s.0.as_f32()];
            }
            TypedOperation::NonStrokeColorDeviceRgb(s) => {
                context.get_mut().fill_cs = ColorSpace::DeviceRgb;
                context.get_mut().fill_color = smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32()];
            }
            TypedOperation::NonStrokeColorCmyk(s) => {
                context.get_mut().fill_cs = ColorSpace::DeviceCmyk;
                context.get_mut().fill_color =
                    smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32(), s.3.as_f32()];
            }
            TypedOperation::LineTo(m) => {
                let last_point = *context.last_point();
                let mut p = Point::new(m.0.as_f64(), m.1.as_f64());
                *(context.last_point_mut()) = p;
                if last_point == p {
                    // Add a small delta so that zero width lines can still have a round stroke.
                    p.x += 0.0001;
                }

                context.path_mut().line_to(p);
            }
            TypedOperation::CubicTo(c) => {
                let p1 = Point::new(c.0.as_f64(), c.1.as_f64());
                let p2 = Point::new(c.2.as_f64(), c.3.as_f64());
                let p3 = Point::new(c.4.as_f64(), c.5.as_f64());

                *(context.last_point_mut()) = p3;

                context.path_mut().curve_to(p1, p2, p3)
            }
            TypedOperation::CubicStartTo(c) => {
                let p1 = *context.last_point();
                let p2 = Point::new(c.0.as_f64(), c.1.as_f64());
                let p3 = Point::new(c.2.as_f64(), c.3.as_f64());

                *(context.last_point_mut()) = p3;

                context.path_mut().curve_to(p1, p2, p3)
            }
            TypedOperation::CubicEndTo(c) => {
                let p2 = Point::new(c.0.as_f64(), c.1.as_f64());
                let p3 = Point::new(c.2.as_f64(), c.3.as_f64());

                *(context.last_point_mut()) = p3;

                context.path_mut().curve_to(p2, p3, p3)
            }
            TypedOperation::ClosePath(_) => {
                context.path_mut().close_path();

                *(context.last_point_mut()) = *context.sub_path_start();
            }
            TypedOperation::SetGraphicsState(gs) => {
                let gs = ext_g_states
                    .get::<Dict>(&gs.0)
                    .warn_none(&format!("failed to get extgstate {}", gs.0.as_str()))
                    .unwrap_or_default();

                handle_gs(&gs, context);
            }
            TypedOperation::StrokePath(_) => {
                stroke_path(context, device);
            }
            TypedOperation::EndPath(_) => {
                if let Some(clip) = *context.clip() {
                    device.set_transform(context.get().affine);
                    device.push_clip(context.path(), clip);

                    *(context.clip_mut()) = None;
                    context.get_mut().n_clips += 1;
                }

                context.path_mut().truncate(0);
            }
            TypedOperation::NonStrokeColor(c) => {
                let fill_c = &mut context.get_mut().fill_color;
                fill_c.truncate(0);

                for e in c.0 {
                    fill_c.push(e.as_f32());
                }
            }
            TypedOperation::StrokeColor(c) => {
                let stroke_c = &mut context.get_mut().stroke_color;
                stroke_c.truncate(0);

                for e in c.0 {
                    stroke_c.push(e.as_f32());
                }
            }
            TypedOperation::ClipNonZero(_) => {
                *(context.clip_mut()) = Some(Fill::NonZero);
            }
            TypedOperation::ClipEvenOdd(_) => {
                *(context.clip_mut()) = Some(Fill::EvenOdd);
            }
            TypedOperation::RestoreState(_) => {
                let mut num_clips = context.get().n_clips;
                context.restore_state();
                let target_clips = context.get().n_clips;

                while num_clips > target_clips {
                    device.pop_clip();
                    num_clips -= 1;
                }
            }
            TypedOperation::FlatnessTolerance(_) => {
                // Ignore for now.
            }
            TypedOperation::ColorSpaceStroke(c) => {
                context.get_mut().stroke_cs = handle_cs(c.0);
            }
            TypedOperation::ColorSpaceNonStroke(c) => {
                context.get_mut().fill_cs = handle_cs(c.0);
            }
            TypedOperation::DashPattern(p) => {
                context.get_mut().dash_offset = p.1.as_f32();
                context.get_mut().dash_array = p.0.iter::<f32>().collect();
            }
            TypedOperation::RenderingIntent(_) => {
                // Ignore for now.
            }
            TypedOperation::NonStrokeColorNamed(n) => {
                if n.1.is_none() {
                    context.get_mut().fill_color = n.0.into_iter().map(|n| n.as_f32()).collect();
                } else {
                    warn!("named color spaces are not supported!");
                }
            }
            TypedOperation::StrokeColorNamed(n) => {
                if n.1.is_none() {
                    context.get_mut().stroke_color = n.0.into_iter().map(|n| n.as_f32()).collect();
                } else {
                    warn!("named color spaces are not supported!");
                }
            }
            TypedOperation::BeginMarkedContentWithProperties(_) => {}
            TypedOperation::MarkedContentPointWithProperties(_) => {}
            TypedOperation::EndMarkedContent(_) => {}
            TypedOperation::MarkedContentPoint(_) => {}
            TypedOperation::BeginMarkedContent(_) => {}
            TypedOperation::BeginText(_) => {
                context.get_mut().text_state.text_matrix = Affine::IDENTITY;
                context.get_mut().text_state.text_line_matrix = Affine::IDENTITY;
            }
            TypedOperation::SetTextMatrix(m) => {
                let m = Affine::new([
                    m.0.as_f64(),
                    m.1.as_f64(),
                    m.2.as_f64(),
                    m.3.as_f64(),
                    m.4.as_f64(),
                    m.5.as_f64(),
                ]);
                context.get_mut().text_state.text_line_matrix = m;
                context.get_mut().text_state.text_matrix = m;
            }
            TypedOperation::EndText(_) => {
                let has_outline = context
                    .get()
                    .text_state
                    .clip_paths
                    .segments()
                    .next()
                    .is_some();

                if has_outline {
                    device.set_transform(context.get().affine);
                    device.push_clip(&context.get().text_state.clip_paths, Fill::NonZero);
                    context.get_mut().n_clips += 1;
                }

                context.get_mut().text_state.clip_paths.truncate(0);
            }
            TypedOperation::TextFont(t) => {
                let font = context.get_font(&fonts, t.0);
                context.get_mut().text_state.font = Some((font, t.1.as_f32()));
            }
            TypedOperation::ShowText(s) => {
                let font = context.get().text_state.font();
                show_text_string(context, device, s.0, &font);
            }
            TypedOperation::ShowTexts(s) => {
                let font = context.get().text_state.font();

                for obj in s.0.iter::<Object>() {
                    if let Ok(adjustment) = obj.clone().cast::<f32>() {
                        context.get_mut().text_state.apply_adjustment(adjustment, font.is_horizontal());
                    } else if let Ok(text) = obj.cast::<String>() {
                        show_text_string(context, device, text, &font);
                    }
                }
            }
            TypedOperation::HorizontalScaling(h) => {
                context.get_mut().text_state.horizontal_scaling = h.0.as_f32();
            }
            TypedOperation::TextLeading(tl) => {
                context.get_mut().text_state.leading = tl.0.as_f32();
            }
            TypedOperation::CharacterSpacing(c) => {
                context.get_mut().text_state.char_space = c.0.as_f32()
            }
            TypedOperation::WordSpacing(w) => {
                context.get_mut().text_state.word_space = w.0.as_f32();
            }
            TypedOperation::NextLine(n) => {
                let (tx, ty) = (n.0.as_f64(), n.1.as_f64());
                next_line(context, tx, ty)
            }
            TypedOperation::NextLineUsingLeading(_) => {
                next_line(context, 0.0, -context.get().text_state.leading as f64);
            }
            TypedOperation::NextLineAndShowText(n) => {
                let font = context.get().text_state.font();

                next_line(context, 0.0, -context.get().text_state.leading as f64);
                show_text_string(context, device, n.0, &font)
            }
            TypedOperation::TextRenderingMode(r) => {
                let mode = match r.0.as_i32() {
                    0 => TextRenderingMode::Fill,
                    1 => TextRenderingMode::Stroke,
                    2 => TextRenderingMode::FillStroke,
                    3 => TextRenderingMode::Invisible,
                    4 => TextRenderingMode::FillAndClip,
                    5 => TextRenderingMode::StrokeAndClip,
                    6 => TextRenderingMode::FillAndStrokeAndClip,
                    7 => TextRenderingMode::Clip,
                    _ => {
                        warn!("unknown text rendering mode {}", r.0.as_i32());

                        TextRenderingMode::Fill
                    }
                };

                context.get_mut().text_state.render_mode = mode;
            }
            TypedOperation::NextLineAndSetLeading(n) => {
                let (tx, ty) = (n.0.as_f64(), n.1.as_f64());
                context.get_mut().text_state.leading = -ty as f32;
                next_line(context, tx, ty)
            }
            _ => {
                println!("{:?}", op);
            }
        }
    }

    for _ in 0..context.get().n_clips {
        device.pop_clip();
    }
}

fn next_line(ctx: &mut Context, tx: f64, ty: f64) {
    let new_matrix = ctx.get_mut().text_state.text_line_matrix * Affine::translate((tx, ty));
    ctx.get_mut().text_state.text_line_matrix = new_matrix;
    ctx.get_mut().text_state.text_matrix = new_matrix;
}

fn show_text_string(ctx: &mut Context, device: &mut impl Device, text: String, font: &Font) {
    let code_len = font.code_len();
    for b in text.get().chunks(code_len) {
        let code = match code_len {
            1 => b[0] as u16,
            2 => u16::from_be_bytes([b[0], b[1]]),
            _ => unimplemented!(),
        };

        let glyph = font.map_code(code);
        show_glyph(ctx, device, glyph, &font);

        ctx.get_mut()
            .text_state
            .apply_glyph_width(font.code_width(code), code, code_len, font.is_horizontal());
    }
}

fn show_glyph(ctx: &mut Context, device: &mut impl Device, glyph: GlyphId, font: &Font) {
    let t = ctx.get().text_transform() * Affine::scale(1.0 / 1000.0);
    let outline = t * font.outline_glyph(glyph);

    match ctx.get().text_state.render_mode {
        TextRenderingMode::Fill => fill_path_impl(ctx, device, Some(&outline), None),
        TextRenderingMode::Stroke => stroke_path_impl(ctx, device, Some(&outline), None),
        TextRenderingMode::FillStroke => {
            fill_path_impl(ctx, device, Some(&outline), None);
            stroke_path_impl(ctx, device, Some(&outline), None);
        }
        TextRenderingMode::Invisible => {}
        TextRenderingMode::Clip => {
            clip_impl(ctx, &outline);
        }
        TextRenderingMode::FillAndClip => {
            clip_impl(ctx, &outline);
            fill_path_impl(ctx, device, Some(&outline), None);
        }
        TextRenderingMode::StrokeAndClip => {
            clip_impl(ctx, &outline);
            stroke_path_impl(ctx, device, Some(&outline), None);
        }
        TextRenderingMode::FillAndStrokeAndClip => {
            clip_impl(ctx, &outline);
            fill_path_impl(ctx, device, Some(&outline), None);
            stroke_path_impl(ctx, device, Some(&outline), None);
        }
    }
}

fn handle_cs(key: Name) -> ColorSpace {
    match key.as_ref() {
        DEVICE_RGB => ColorSpace::DeviceRgb,
        DEVICE_GRAY => ColorSpace::DeviceGray,
        DEVICE_CMYK => ColorSpace::DeviceCmyk,
        _ => {
            warn!("unsupported color space {}", key.as_str());

            ColorSpace::DeviceGray
        }
    }
}

fn handle_gs(dict: &Dict, context: &mut Context) {
    for key in dict.keys() {
        handle_gs_single(dict, key, context).warn_none(&format!(
            "invalid value in graphics state for {}",
            key.as_str()
        ));
    }
}

fn handle_gs_single(dict: &Dict, key: &Name, context: &mut Context) -> Option<()> {
    // TODO Can we use constants here somehow?
    match key.as_str() {
        "LW" => context.get_mut().line_width = dict.get::<f32>(&key)?,
        "LC" => context.get_mut().line_cap = convert_line_cap(LineCap(dict.get::<Number>(&key)?)),
        "LJ" => {
            context.get_mut().line_join = convert_line_join(LineJoin(dict.get::<Number>(&key)?))
        }
        "ML" => context.get_mut().miter_limit = dict.get::<f32>(&key)?,
        "CA" => context.get_mut().stroke_alpha = dict.get::<f32>(&key)?,
        "ca" => context.get_mut().fill_alpha = dict.get::<f32>(&key)?,
        "Type" => {}
        _ => {}
    }

    Some(())
}

fn fill_path(context: &mut Context, device: &mut impl Device) {
    fill_path_impl(context, device, None, None);
    // TODO: Where in spec?
    context.path_mut().truncate(0);
}

fn stroke_path(context: &mut Context, device: &mut impl Device) {
    stroke_path_impl(context, device, None, None);
    context.path_mut().truncate(0);
}

fn fill_stroke_path(context: &mut Context, device: &mut impl Device) {
    fill_path_impl(context, device, None, None);
    stroke_path_impl(context, device, None, None);
    context.path_mut().truncate(0);
}

fn clip_impl(context: &mut Context, outline: &BezPath) {
    let has_outline = outline.segments().next().is_some();

    if has_outline {
        context.get_mut().text_state.clip_paths.extend(outline);
    }
}

fn fill_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    path: Option<&BezPath>,
    transform: Option<Affine>,
) {
    let color = Color::from_pdf(
        context.get().fill_cs,
        &context.get().fill_color,
        context.get().fill_alpha,
    );

    device.set_paint(color);
    device.set_transform(transform.unwrap_or(context.get().affine));
    device.fill_path(path.unwrap_or(context.path()), &context.fill_props());
}

fn stroke_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    path: Option<&BezPath>,
    transform: Option<Affine>,
) {
    let color = Color::from_pdf(
        context.get().stroke_cs,
        &context.get().stroke_color,
        context.get().stroke_alpha,
    );
    device.set_paint(color);
    device.set_transform(transform.unwrap_or(context.get().affine));
    device.stroke_path(path.unwrap_or(context.path()), &context.stroke_props());
}
