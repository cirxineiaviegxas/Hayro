use crate::font::base::BaseFont;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_FONT, TYPE};
use hayro_syntax::object::name::Name;
use std::fmt::Debug;
use std::sync::Arc;
use crate::font::blob::{FontBlob, ROBOTO};

mod base;
mod blob;
mod encodings;
mod glyph_list;

#[derive(Clone, Debug)]
pub struct Font(Arc<FontType>);

impl Font {
    pub fn new(dict: &Dict) -> Option<Self> {
        let f_type = match dict.get::<Name>(TYPE)?.as_str().as_bytes() {
            b"Type1" => FontType::Type1Font(Type1Font::new(dict)),
            _ => unimplemented!(),
        };

        Some(Self(Arc::new(f_type)))
    }
}

#[derive(Debug)]
enum FontType {
    Type1Font(Type1Font),
}

#[derive(Debug)]
struct Type1Font {
    base_font: Option<BaseFont>,
    blob: FontBlob,
}

impl Type1Font {
    pub fn new(dict: &Dict) -> Type1Font {
        let (base_font, blob) = if let Some(n) = dict.get::<Name>(BASE_FONT) {
            match n.get().as_ref() {
                b"Helvetica" => (BaseFont::Helvetica, ROBOTO.clone()),
                _ => unimplemented!(),
            }
        } else {
            unimplemented!()
        };

        Self {
            base_font: Some(base_font),
            blob,
        }
    }
}

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
