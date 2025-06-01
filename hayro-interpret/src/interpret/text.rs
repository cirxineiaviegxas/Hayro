use crate::context::Context;
use crate::device::Device;
use crate::font::{Font, TextRenderingMode, UNITS_PER_EM};
use crate::glyph::Glyph;
use crate::interpret::path::{clip_impl, fill_path_impl, set_device_paint, stroke_path_impl};
use hayro_syntax::document::page::Resources;
use hayro_syntax::object::dict::keys::P;
use hayro_syntax::object::string;
use kurbo::{Affine, Vec2};
use log::warn;
use skrifa::GlyphId;
use yoke::Yokeable;

pub(crate) fn show_text_string<'a>(
    ctx: &mut Context<'a>,
    device: &mut impl Device,
    resources: &Resources<'a>,
    text: string::String,
) {
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

        let glyph = font.get_glyph(
            font.map_code(code),
            ctx,
            resources,
            font.origin_displacement(code),
        );
        show_glyph(ctx, device, &glyph);

        ctx.get_mut().text_state.apply_code_advance(code);
    }
}

pub(crate) fn next_line(ctx: &mut Context, tx: f64, ty: f64) {
    let new_matrix = ctx.get_mut().text_state.text_line_matrix * Affine::translate((tx, ty));
    ctx.get_mut().text_state.text_line_matrix = new_matrix;
    ctx.get_mut().text_state.text_matrix = new_matrix;
}

pub(crate) fn show_glyph<'a>(ctx: &mut Context<'a>, device: &mut impl Device, glyph: &Glyph<'a>) {
    device.set_transform(ctx.get().ctm);

    device.set_stroke_properties(&ctx.stroke_props());
    device.set_fill_properties(&ctx.fill_props());

    match ctx.get().text_state.render_mode {
        TextRenderingMode::Fill => {
            set_device_paint(ctx, device, false);
            device.fill_glyph(glyph);
        }
        TextRenderingMode::Stroke => {
            set_device_paint(ctx, device, true);
            device.stroke_glyph(glyph);
        }
        TextRenderingMode::FillStroke => {
            set_device_paint(ctx, device, false);
            device.fill_glyph(glyph);
            set_device_paint(ctx, device, true);
            device.stroke_glyph(glyph);
        }
        TextRenderingMode::Invisible => {}
        TextRenderingMode::Clip => {
            clip_impl(ctx, glyph, glyph.glyph_transform());
        }
        TextRenderingMode::FillAndClip => {
            set_device_paint(ctx, device, false);
            clip_impl(ctx, glyph, glyph.glyph_transform());
            device.fill_glyph(glyph);
        }
        TextRenderingMode::StrokeAndClip => {
            set_device_paint(ctx, device, true);
            clip_impl(ctx, glyph, glyph.glyph_transform());
            device.stroke_glyph(glyph);
        }
        TextRenderingMode::FillAndStrokeAndClip => {
            clip_impl(ctx, glyph, glyph.glyph_transform());
            set_device_paint(ctx, device, false);
            device.fill_glyph(glyph);
            set_device_paint(ctx, device, true);
            device.stroke_glyph(glyph);
        }
    }
}
