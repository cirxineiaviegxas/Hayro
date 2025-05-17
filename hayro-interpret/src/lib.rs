use crate::convert::{convert_line_cap, convert_line_join};
use crate::device::{ClipPath, Device, ReplayInstruction};
use hayro_syntax::content::ops::{LineCap, LineJoin, TypedOperation};
use hayro_syntax::object::Object;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{COLORSPACE, EXT_G_STATE, FONT, PATTERN, SHADING, XOBJECT};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::number::Number;
use hayro_syntax::object::rect::Rect;
use hayro_syntax::object::stream::Stream;
use hayro_syntax::object::string::String;
use kurbo::{Affine, BezPath, Cap, Join, Point, Shape, Vec2};
use log::warn;
use peniko::Fill;
use peniko::color::palette::css::GREEN;
use skrifa::GlyphId;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;

pub mod color;
pub mod context;
mod convert;
pub mod device;
mod font;
pub mod pattern;
pub mod shading;
mod state;
mod util;
pub mod x_object;

use crate::color::{Color, ColorSpace};
use crate::context::Context;
use crate::font::type3::Type3GlyphDescription;
use crate::font::{Font, GlyphDescription, TextRenderingMode};
use crate::pattern::ShadingPattern;
use crate::shading::Shading;
use crate::util::OptionLog;
use crate::x_object::{ImageXObject, XObject, draw_image_xobject, draw_xobject};

#[derive(Clone, Debug)]
pub struct StrokeProps {
    pub line_width: f32,
    pub line_cap: Cap,
    pub line_join: Join,
    pub miter_limit: f32,
    pub dash_array: SmallVec<[f32; 4]>,
    pub dash_offset: f32,
}

#[derive(Clone, Debug)]
pub struct FillProps {
    pub fill_rule: Fill,
}

pub fn interpret<'a, 'b>(
    ops: impl Iterator<Item = TypedOperation<'b>>,
    resources: &Dict<'a>,
    context: &mut Context<'a>,
    device: &mut impl Device,
) {
    let ext_g_states = resources.get::<Dict>(EXT_G_STATE).unwrap_or_default();
    let fonts = resources.get::<Dict>(FONT).unwrap_or_default();
    let color_spaces = resources.get::<Dict>(COLORSPACE).unwrap_or_default();
    let x_objects = resources.get::<Dict>(XOBJECT).unwrap_or_default();
    let patterns = resources.get::<Dict>(PATTERN).unwrap_or_default();
    let shadings = resources.get::<Dict>(SHADING).unwrap_or_default();

    save_sate(context);

    for op in ops {
        match op {
            TypedOperation::SaveState(_) => save_sate(context),
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
                let rect = kurbo::Rect::new(
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
                    device.push_layer(
                        Some(&ClipPath {
                            path: context.path().clone(),
                            fill: clip,
                        }),
                        1.0,
                    );

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
            TypedOperation::RestoreState(_) => restore_state(context, device),
            TypedOperation::FlatnessTolerance(_) => {
                // Ignore for now.
            }
            TypedOperation::ColorSpaceStroke(c) => {
                let cs = if let Some(named) = ColorSpace::new_from_name(c.0.clone()) {
                    named
                } else {
                    context.get_color_space(&color_spaces, c.0)
                };

                cs.set_initial_color(&mut context.get_mut().stroke_color);
                context.get_mut().stroke_cs = cs;
            }
            TypedOperation::ColorSpaceNonStroke(c) => {
                let cs = if let Some(named) = ColorSpace::new_from_name(c.0.clone()) {
                    named
                } else {
                    context.get_color_space(&color_spaces, c.0)
                };

                cs.set_initial_color(&mut context.get_mut().fill_color);
                context.get_mut().fill_cs = cs;
            }
            TypedOperation::DashPattern(p) => {
                context.get_mut().dash_offset = p.1.as_f32();
                // kurbo apparently cannot properly deal with offsets that are exactly 0.
                context.get_mut().dash_array =
                    p.0.iter::<f32>()
                        .map(|n| if n == 0.0 { 0.01 } else { n })
                        .collect();
            }
            TypedOperation::RenderingIntent(_) => {
                // Ignore for now.
            }
            TypedOperation::NonStrokeColorNamed(n) => {
                if let Some(name) = n.1 {
                    let pattern =
                        ShadingPattern::new(&patterns.get::<Dict>(&name).unwrap()).unwrap();
                    context.get_mut().fill_pattern = Some(pattern);
                } else {
                    context.get_mut().fill_color = n.0.into_iter().map(|n| n.as_f32()).collect();
                }
            }
            TypedOperation::StrokeColorNamed(n) => {
                if let Some(name) = n.1 {
                    let pattern =
                        ShadingPattern::new(&patterns.get::<Dict>(&name).unwrap()).unwrap();
                    context.get_mut().stroke_pattern = Some(pattern);
                } else {
                    context.get_mut().stroke_color = n.0.into_iter().map(|n| n.as_f32()).collect();
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
                    device.push_layer(
                        Some(&ClipPath {
                            path: context.get().text_state.clip_paths.clone(),
                            fill: Fill::NonZero,
                        }),
                        1.0,
                    );
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
                    if let Some(adjustment) = obj.clone().cast::<f32>() {
                        context
                            .get_mut()
                            .text_state
                            .apply_adjustment(adjustment, font.is_horizontal());
                    } else if let Some(text) = obj.cast::<String>() {
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
            TypedOperation::ShapeGlyph(_) => {}
            TypedOperation::XObject(x) => {
                if let Some(x_object) = XObject::new(&x_objects.get::<Stream>(&x.0).unwrap()) {
                    draw_xobject(&x_object, context, device);
                }
            }
            TypedOperation::InlineImage(i) => {
                if let Some(x_object) = ImageXObject::new(&i.0) {
                    draw_image_xobject(&x_object, context, device)
                }
            }
            TypedOperation::TextRise(t) => {
                context.get_mut().text_state.rise = t.0.as_f32();
            }
            TypedOperation::Shading(s) => {
                let shading_dict = shadings.get::<Dict>(&s.0).unwrap();
                let shading_pattern = {
                    let shading = Shading::new(&shading_dict).unwrap();

                    ShadingPattern {
                        shading: Arc::new(shading),
                        matrix: Affine::IDENTITY,
                    }
                };

                context.save_state();
                context.push_root_transform();
                let st = context.get_mut();
                st.fill_pattern = Some(shading_pattern);
                st.fill_cs = ColorSpace::Pattern;

                let bbox = context.bbox().to_path(0.1);
                let inverted_bbox = context.get().affine.inverse() * bbox;
                fill_path_impl(
                    context,
                    device,
                    Some(&GlyphDescription::Path(inverted_bbox)),
                    None,
                );

                context.restore_state();
            }
            _ => {
                println!("{:?}", op);
            }
        }
    }

    restore_state(context, device);
}

fn save_sate(ctx: &mut Context) {
    ctx.save_state();
}

fn restore_state(ctx: &mut Context, device: &mut impl Device) {
    let mut num_clips = ctx.get().n_clips;
    ctx.restore_state();
    let target_clips = ctx.get().n_clips;

    while num_clips > target_clips {
        device.pop();
        num_clips -= 1;
    }
}

fn next_line(ctx: &mut Context, tx: f64, ty: f64) {
    let new_matrix = ctx.get_mut().text_state.text_line_matrix * Affine::translate((tx, ty));
    ctx.get_mut().text_state.text_line_matrix = new_matrix;
    ctx.get_mut().text_state.text_matrix = new_matrix;
}

fn show_text_string<'a>(
    ctx: &mut Context<'a>,
    device: &mut impl Device,
    text: String,
    font: &Font<'a>,
) {
    let code_len = font.code_len();
    for b in text.get().chunks(code_len) {
        let code = match code_len {
            1 => b[0] as u16,
            2 => u16::from_be_bytes([b[0], b[1]]),
            _ => unimplemented!(),
        };

        let glyph = font.map_code(code);
        show_glyph(ctx, device, glyph, font, font.origin_displacement(code));

        ctx.get_mut().text_state.apply_glyph_width(
            font.code_advance(code),
            code,
            code_len,
            font.is_horizontal(),
        );
    }
}

fn show_glyph<'a>(
    ctx: &mut Context<'a>,
    device: &mut impl Device,
    glyph: GlyphId,
    font: &Font<'a>,
    origin_displacement: Vec2,
) {
    let t = ctx.get().text_transform()
        * Affine::scale(1.0 / 1000.0)
        * Affine::translate(origin_displacement);
    let glyph_description = match font.render_glyph(glyph, ctx) {
        GlyphDescription::Path(path) => GlyphDescription::Path(t * path),
        GlyphDescription::Type3(mut desc) => {
            desc.1 = t * desc.1;
            GlyphDescription::Type3(desc)
        }
    };

    match ctx.get().text_state.render_mode {
        TextRenderingMode::Fill => fill_path_impl(ctx, device, Some(&glyph_description), None),
        TextRenderingMode::Stroke => stroke_path_impl(ctx, device, Some(&glyph_description), None),
        TextRenderingMode::FillStroke => {
            fill_path_impl(ctx, device, Some(&glyph_description), None);
            stroke_path_impl(ctx, device, Some(&glyph_description), None);
        }
        TextRenderingMode::Invisible => {}
        TextRenderingMode::Clip => {
            clip_impl(ctx, &glyph_description);
        }
        TextRenderingMode::FillAndClip => {
            clip_impl(ctx, &glyph_description);
            fill_path_impl(ctx, device, Some(&glyph_description), None);
        }
        TextRenderingMode::StrokeAndClip => {
            clip_impl(ctx, &glyph_description);
            stroke_path_impl(ctx, device, Some(&glyph_description), None);
        }
        TextRenderingMode::FillAndStrokeAndClip => {
            clip_impl(ctx, &glyph_description);
            fill_path_impl(ctx, device, Some(&glyph_description), None);
            stroke_path_impl(ctx, device, Some(&glyph_description), None);
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
        "LW" => context.get_mut().line_width = dict.get::<f32>(key)?,
        "LC" => context.get_mut().line_cap = convert_line_cap(LineCap(dict.get::<Number>(key)?)),
        "LJ" => context.get_mut().line_join = convert_line_join(LineJoin(dict.get::<Number>(key)?)),
        "ML" => context.get_mut().miter_limit = dict.get::<f32>(key)?,
        "CA" => context.get_mut().stroke_alpha = dict.get::<f32>(key)?,
        "ca" => context.get_mut().fill_alpha = dict.get::<f32>(key)?,
        "Type" => {}
        _ => {}
    }

    Some(())
}

// TODO: Apply bbox if shading has one!

fn fill_path(context: &mut Context, device: &mut impl Device) {
    fill_path_impl(context, device, None, None);
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

fn clip_impl(context: &mut Context, outline: &GlyphDescription) {
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

fn fill_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    path: Option<&GlyphDescription>,
    transform: Option<Affine>,
) {
    let base_transform = transform.unwrap_or(context.get().affine);
    device.set_transform(base_transform);

    let need_pop = handle_paint(context, device, base_transform, false);

    match path {
        None => device.fill_path(context.path(), &context.fill_props()),
        Some(GlyphDescription::Path(path)) => device.fill_path(path, &context.fill_props()),
        Some(GlyphDescription::Type3(t3)) => run_t3_instructions(device, t3, base_transform * t3.1),
    };

    if need_pop {
        device.pop();
    }
}

fn handle_paint(    context: &mut Context,
                      device: &mut impl Device, base_transform: Affine, is_stroke: bool) -> bool {
    let (cs, pattern, color, alpha) = if is_stroke {
        let s = context.get();
        (s.stroke_cs.clone(), s.stroke_pattern.clone(), s.stroke_color.clone(), s.stroke_alpha)
    }   else {
        let s = context.get();
        (s.fill_cs.clone(), s.fill_pattern.clone(), s.fill_color.clone(), s.fill_alpha)
    };
    
    let clip_path = if matches!(cs, ColorSpace::Pattern) {
        let mut pattern = pattern.unwrap();
        pattern.matrix = *context.root_transform() * pattern.matrix;
        let bbox = pattern.shading.bbox;
        device.set_shading_paint(pattern);

        bbox
    } else {
        let color = Color::from_pdf(
            cs,
            color,
            alpha,
        );

        device.set_paint(color);

        None
    };

    if let Some(clip_path) = clip_path {
        // Temporary hack, because currently a clip path will always assume the transform used
        // by `set_transform`.
        device.set_transform(*context.root_transform());
        device.push_layer(
            Some(&ClipPath {
                path: clip_path.get().to_path(0.1),
                fill: Fill::NonZero,
            }),
            1.0,
        );
        device.set_transform(base_transform);
    }
    
    clip_path.is_some()
}

fn stroke_path_impl(
    context: &mut Context,
    device: &mut impl Device,
    path: Option<&GlyphDescription>,
    transform: Option<Affine>,
) {
    let base_transform = transform.unwrap_or(context.get().affine);
    device.set_transform(base_transform);

    let need_pop = handle_paint(context, device, base_transform, true);

    match path {
        None => device.stroke_path(context.path(), &context.stroke_props()),
        Some(GlyphDescription::Path(path)) => device.stroke_path(path, &context.stroke_props()),
        Some(GlyphDescription::Type3(t3)) => run_t3_instructions(device, t3, base_transform * t3.1),
    };

    if need_pop {
        device.pop();
    }
}

fn run_t3_instructions(
    device: &mut impl Device,
    description: &Type3GlyphDescription,
    initial_transform: Affine,
) {
    for instruction in &description.0 {
        match instruction {
            ReplayInstruction::SetTransform { affine } => {
                device.set_transform(initial_transform * *affine);
            }
            ReplayInstruction::SetPaint { .. } => {}
            ReplayInstruction::StrokePath { path, stroke_props } => {
                device.stroke_path(path, stroke_props);
            }
            ReplayInstruction::FillPath { path, fill_props } => {
                device.fill_path(path, fill_props);
            }
            ReplayInstruction::PushLayer { clip, opacity } => {
                device.push_layer(clip.as_ref(), *opacity)
            }
            ReplayInstruction::PopClip => device.pop(),
            ReplayInstruction::ApplyMask { mask } => {
                device.apply_mask(mask);
            }
            ReplayInstruction::DrawImage {
                image_data,
                width,
                height,
                is_stencil,
                quality,
            } => device.draw_rgba_image(image_data.clone(), *width, *height, *is_stencil, *quality),
            ReplayInstruction::SetRootTransform { affine } => {
                device.set_transform(*affine);
            }
        }
    }
}
