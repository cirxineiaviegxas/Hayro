use crate::font::glyph_list::GLYPH_NAME_MAP;

mod glyph_list;

#[derive(Debug, Clone, Copy)]
pub struct Font();

#[derive(Debug, Clone, Copy, Default)]
pub enum TextRenderingMode {
    #[default]
    Fill,
    Stroke,
    FillStroke,
    Invisible,
    FillAndClip,
    StrokeAndClip,
    FillAndStrokeAndClip,
    Clip,
}

fn test() {
    GLYPH_NAME_MAP.get("AEacute").unwrap();
}