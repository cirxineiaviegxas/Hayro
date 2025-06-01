use hayro_syntax::object::string;
use log::warn;
use skrifa::GlyphId;
use kurbo::{Affine, Vec2};
use crate::context::Context;
use crate::device::{Device, ReplayInstruction};
use crate::font::{Font, GlyphDescription, TextRenderingMode};
use crate::font::type3::Type3GlyphDescription;
use crate::interpret::path::{clip_impl, fill_path_impl, stroke_path_impl};

pub(crate) fn show_text_string<'a>(ctx: &mut Context<'a>, device: &mut impl Device, text: string::String) {
    let Some(font) = ctx.get().text_state.font.clone() else {
        warn!("tried to show text without active font");

        return;
    };

    let code_len = font.code_len();
    for b in text.get().chunks(code_len) {
        let code = match code_len {
            1 => b[0] as u16,
            2 => u16::from_be_bytes([b[0], b[1]]),
            _ => unimplemented!(),
        };

        let glyph = font.map_code(code);
        show_glyph(ctx, device, glyph, &font, font.origin_displacement(code));

        ctx.get_mut().text_state.apply_code_advance(code);
    }
}

pub(crate) fn next_line(ctx: &mut Context, tx: f64, ty: f64) {
    let new_matrix = ctx.get_mut().text_state.text_line_matrix * Affine::translate((tx, ty));
    ctx.get_mut().text_state.text_line_matrix = new_matrix;
    ctx.get_mut().text_state.text_matrix = new_matrix;
}

pub(crate) fn show_glyph<'a>(
    ctx: &mut Context<'a>,
    device: &mut impl Device,
    glyph: GlyphId,
    font: &Font<'a>,
    origin_displacement: Vec2,
) {
    let t = ctx.get().text_state.full_transform()
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

pub(crate) fn run_t3_instructions(
    device: &mut impl Device,
    description: &Type3GlyphDescription,
    initial_transform: Affine,
) {
    for instruction in &description.0 {
        match instruction {
            ReplayInstruction::SetTransform { affine } => {
                device.set_transform(initial_transform * *affine);
            }
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
            ReplayInstruction::DrawImage { image } => device.draw_rgba_image(image.clone()),
            ReplayInstruction::DrawStencil { stencil_image } => {
                device.draw_stencil_image(stencil_image.clone())
            }
            ReplayInstruction::SetPaintTransform { affine } => {
                device.set_paint_transform(*affine);
            }
        }
    }
}