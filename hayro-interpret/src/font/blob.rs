use once_cell::sync::Lazy;
use skrifa::charmap::Charmap;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::{FontRef, MetadataProvider, OutlineGlyphCollection};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use yoke::{Yoke, Yokeable};

pub(crate) static ROBOTO_REGULAR: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/Roboto-Regular.ttf"
    )))
});

pub(crate) static ROBOTO_BOLD: Lazy<FontBlob> =
    Lazy::new(|| FontBlob::new(Arc::new(include_bytes!("../../../assets/Roboto-Bold.ttf"))));

pub(crate) static ROBOTO_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/Roboto-Italic.ttf"
    )))
});

pub(crate) static ROBOTO_BOLD_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/Roboto-BoldItalic.ttf"
    )))
});

pub(crate) static COURIER_PRIME_REGULAR: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/CourierPrime-Regular.ttf"
    )))
});

pub(crate) static COURIER_PRIME_BOLD: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/CourierPrime-Bold.ttf"
    )))
});

pub(crate) static COURIER_PRIME_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/CourierPrime-Italic.ttf"
    )))
});

pub(crate) static COURIER_PRIME_BOLD_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/CourierPrime-BoldItalic.ttf"
    )))
});

pub(crate) static EBGARAMOND_REGULAR: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/EBGaramond-Regular.ttf"
    )))
});

pub(crate) static EBGARAMOND_BOLD: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/EBGaramond-Bold.ttf"
    )))
});

pub(crate) static EBGARAMOND_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/EBGaramond-Italic.ttf"
    )))
});

pub(crate) static EBGARAMOND_BOLD_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/EBGaramond-BoldItalic.ttf"
    )))
});

pub(crate) static DEJAVU_SANS: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(Arc::new(include_bytes!(
        "../../../assets/DejaVuSansSubset.ttf"
    )))
});

pub(crate) static TUFFY: Lazy<FontBlob> =
    Lazy::new(|| FontBlob::new(Arc::new(include_bytes!("../../../assets/TuffySubset.ttf"))));

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
                    glyph_metrics: font_ref.glyph_metrics(Size::new(1.0), LocationRef::default()),
                    charmap: font_ref.charmap(),
                }
            });

        Self(Arc::new(font_ref_yoke))
    }

    pub fn font_ref(&self) -> &FontRef {
        &self.0.as_ref().get().font_ref
    }

    pub fn glyph_metrics(&self) -> &GlyphMetrics {
        &self.0.as_ref().get().glyph_metrics
    }

    pub fn outline_glyphs(&self) -> &OutlineGlyphCollection {
        &self.0.as_ref().get().outline_glyphs
    }

    pub fn charmap(&self) -> &Charmap {
        &self.0.as_ref().get().charmap
    }
}

#[derive(Yokeable, Clone)]
struct FontRefYoke<'a> {
    pub font_ref: FontRef<'a>,
    pub glyph_metrics: GlyphMetrics<'a>,
    pub outline_glyphs: OutlineGlyphCollection<'a>,
    pub charmap: Charmap<'a>,
}
