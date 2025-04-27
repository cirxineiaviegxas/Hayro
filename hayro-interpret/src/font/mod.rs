use crate::font::base::BaseFont;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::{FontRef, MetadataProvider, OutlineGlyphCollection};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use yoke::{Yoke, Yokeable};

mod base;
mod blob;
mod encodings;
mod glyph_list;

#[derive(Clone, Debug)]
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
