use once_cell::sync::Lazy;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::{FontRef, MetadataProvider, OutlineGlyphCollection};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use yoke::{Yoke, Yokeable};

pub(crate) static ROBOTO: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/Roboto-Regular.ttf"
    )))
});

type FontData = Arc<dyn AsRef<[u8]> + Send + Sync>;
type FontYoke = Yoke<FontRefYoke<'static>, FontData>;

// TODO: Wrap in Arc?
#[derive(Clone)]
pub struct FontBlob(Arc<FontYoke>);

impl Debug for FontBlob {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Font {{ .. }}")
    }
}

impl FontBlob {
    pub fn new(data: FontData) -> Self {
        let font_ref_yoke =
            Yoke::<FontRefYoke<'static>, FontData>::attach_to_cart(data.clone(), |data| {
                let font_ref = FontRef::from_index(data.as_ref(), 0).unwrap();
                FontRefYoke {
                    font_ref: font_ref.clone(),
                    outline_glyphs: font_ref.outline_glyphs(),
                    glyph_metrics: font_ref
                        // PDF fonts assume a upem of 1000, so setting this here saves us some
                        // work later.
                        .glyph_metrics(Size::new(1000.0), LocationRef::default()),
                }
            });

        Self(Arc::new(font_ref_yoke))
    }
}

#[derive(Yokeable, Clone)]
struct FontRefYoke<'a> {
    pub font_ref: FontRef<'a>,
    pub glyph_metrics: GlyphMetrics<'a>,
    pub outline_glyphs: OutlineGlyphCollection<'a>,
}
