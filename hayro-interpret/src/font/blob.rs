use crate::font::{OutlinePath, UNITS_PER_EM};
use kurbo::BezPath;
use once_cell::sync::Lazy;
use skrifa::charmap::Charmap;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::outline::DrawSettings;
use skrifa::{FontRef, GlyphId, MetadataProvider, OutlineGlyphCollection};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use skrifa::raw::TableProvider;
use yoke::{Yoke, Yokeable};

pub(crate) static HELVETICA_REGULAR: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!("/System/Library/Fonts/HelveticaNeue.ttc")),
        0,
    )
});

pub(crate) static HELVETICA_BOLD: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!("/System/Library/Fonts/HelveticaNeue.ttc")),
        1,
    )
});

pub(crate) static HELVETICA_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!("/System/Library/Fonts/HelveticaNeue.ttc")),
        2,
    )
});

pub(crate) static HELVETICA_BOLD_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!("/System/Library/Fonts/HelveticaNeue.ttc")),
        3,
    )
});

pub(crate) static COURIER_REGULAR: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Courier New.ttf"
        )),
        0,
    )
});

pub(crate) static COURIER_BOLD: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Courier New Bold.ttf"
        )),
        0,
    )
});

pub(crate) static COURIER_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Courier New Italic.ttf"
        )),
        0,
    )
});

pub(crate) static COURIER_BOLD_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Courier New Bold Italic.ttf"
        )),
        0,
    )
});

pub(crate) static TIMES_REGULAR: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Times New Roman.ttf" // "../../../assets/EBGaramond-Regular.ttf"
        )),
        0,
    )
});

pub(crate) static TIMES_BOLD: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Times New Roman Bold.ttf"
        )),
        0,
    )
});

pub(crate) static TIMES_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf"
        )),
        0,
    )
});

pub(crate) static TIMES_ROMAN_BOLD_ITALIC: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!(
            "/System/Library/Fonts/Supplemental/Times New Roman Bold Italic.ttf"
        )),
        0,
    )
});

pub(crate) static ZAPF_DINGS_BAT: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!("/System/Library/Fonts/ZapfDingbats.ttf")),
        0,
    )
});

pub(crate) static SYMBOL: Lazy<FontBlob> = Lazy::new(|| {
    FontBlob::new(
        Arc::new(include_bytes!("/System/Library/Fonts/Symbol.ttf")),
        0,
    )
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
    pub fn new(data: FontData, index: u32) -> Self {
        let font_ref_yoke =
            Yoke::<FontRefYoke<'static>, FontData>::attach_to_cart(data.clone(), |data| {
                let font_ref = FontRef::from_index(data.as_ref(), index).unwrap();
                FontRefYoke {
                    font_ref: font_ref.clone(),
                    outline_glyphs: font_ref.outline_glyphs(),
                    glyph_metrics: font_ref
                        .glyph_metrics(Size::new(UNITS_PER_EM), LocationRef::default()),
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

    fn outline_glyphs(&self) -> &OutlineGlyphCollection {
        &self.0.as_ref().get().outline_glyphs
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        let mut path = OutlinePath(BezPath::new());
        let draw_settings = DrawSettings::unhinted(Size::new(UNITS_PER_EM), LocationRef::default());

        let Some(outline) = self.outline_glyphs().get(glyph) else {
            return BezPath::new();
        };

        let _ = outline.draw(draw_settings, &mut path);
        path.0
    }
    
    pub fn num_glyphs(&self) -> u16 {
        self.font_ref().maxp().map(|m| m.num_glyphs()).unwrap_or(0)
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
