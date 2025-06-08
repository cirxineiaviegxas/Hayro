use crate::font::{OutlinePath, UNITS_PER_EM};
use hayro_font_parser::{Matrix, cff, type1};
use kurbo::{Affine, BezPath};
use once_cell::sync::Lazy;
use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::outline::{DrawSettings, HintingInstance, HintingOptions, InterpreterVersion};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider, OutlineGlyphCollection};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use yoke::{Yoke, Yokeable};

pub(crate) static HELVETICA_REGULAR: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSans.pfb"
    )))
    .unwrap()
});

pub(crate) static HELVETICA_BOLD: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSansBold.pfb"
    )))
    .unwrap()
});
pub(crate) static HELVETICA_ITALIC: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSansItalic.pfb"
    )))
    .unwrap()
});
pub(crate) static HELVETICA_BOLD_ITALIC: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSansBoldItalic.pfb"
    )))
    .unwrap()
});
pub(crate) static COURIER_REGULAR: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitFixed.pfb"
    )))
    .unwrap()
});

pub(crate) static COURIER_BOLD: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitFixedBold.pfb"
    )))
    .unwrap()
});

pub(crate) static COURIER_ITALIC: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitFixedItalic.pfb"
    )))
    .unwrap()
});

pub(crate) static COURIER_BOLD_ITALIC: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitFixedBoldItalic.pfb"
    )))
    .unwrap()
});

pub(crate) static TIMES_REGULAR: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSerif.pfb"
    )))
    .unwrap()
});

pub(crate) static TIMES_BOLD: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSerifBold.pfb"
    )))
    .unwrap()
});

pub(crate) static TIMES_ITALIC: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSerifItalic.pfb"
    )))
    .unwrap()
});

pub(crate) static TIMES_ROMAN_BOLD_ITALIC: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSerifBoldItalic.pfb"
    )))
    .unwrap()
});

pub(crate) static ZAPF_DINGS_BAT: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitDingbats.pfb"
    )))
    .unwrap()
});

pub(crate) static SYMBOL: Lazy<CffFontBlob> = Lazy::new(|| {
    CffFontBlob::new(Arc::new(include_bytes!(
        "../../../assets/standard_fonts/FoxitSymbol.pfb"
    )))
    .unwrap()
});

type FontData = Arc<dyn AsRef<[u8]> + Send + Sync>;
type OpenTypeFontYoke = Yoke<OTFYoke<'static>, FontData>;
type CffFontYoke = Yoke<CFFYoke<'static>, FontData>;
type Type1FontYoke = Yoke<Type1Yoke<'static>, FontData>;

#[derive(Clone)]
pub struct Type1FontBlob(Arc<Type1FontYoke>);

impl Debug for Type1FontBlob {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Type1 Font {{ .. }}")
    }
}

impl Type1FontBlob {
    pub fn new(data: FontData) -> Self {
        let yoke = Yoke::<Type1Yoke<'static>, FontData>::attach_to_cart(data.clone(), |data| {
            let table = type1::Table::parse(data.as_ref()).unwrap();
            Type1Yoke { table }
        });

        Self(Arc::new(yoke))
    }

    pub(crate) fn table(&self) -> &type1::Table {
        &self.0.as_ref().get().table
    }

    pub fn outline_glyph(&self, name: &str) -> BezPath {
        let mut path = OutlinePath(BezPath::new());

        self.table().outline(name, &mut path).unwrap_or_default();

        Affine::scale(UNITS_PER_EM as f64) * convert_matrix(self.table().matrix()) * path.0
    }
}

#[derive(Clone)]
pub struct CffFontBlob(Arc<CffFontYoke>);

impl Debug for CffFontBlob {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Type1 Font {{ .. }}")
    }
}

impl CffFontBlob {
    pub fn new(data: FontData) -> Option<Self> {
        let _ = cff::Table::parse(data.as_ref().as_ref())?;

        let yoke = Yoke::<CFFYoke<'static>, FontData>::attach_to_cart(data.clone(), |data| {
            let table = cff::Table::parse(data.as_ref()).unwrap();
            CFFYoke { table }
        });

        Some(Self(Arc::new(yoke)))
    }

    pub(crate) fn table(&self) -> &cff::Table {
        &self.0.as_ref().get().table
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        let mut path = OutlinePath(BezPath::new());

        let Ok(_) = self
            .table()
            .outline(hayro_font_parser::GlyphId(glyph.to_u32() as u16), &mut path)
        else {
            return BezPath::new();
        };

        Affine::scale(UNITS_PER_EM as f64) * convert_matrix(self.table().matrix()) * path.0
    }
}

#[derive(Clone)]
pub struct OpenTypeFontBlob(Arc<OpenTypeFontYoke>);

impl Debug for OpenTypeFontBlob {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "OpenType Font {{ .. }}")
    }
}

impl OpenTypeFontBlob {
    pub fn new(data: FontData, index: u32) -> Option<Self> {
        // Check first whether the font is valid so we can unwrap in the closure.
        let _ = FontRef::from_index(data.as_ref().as_ref(), index).ok()?;

        let font_ref_yoke =
            Yoke::<OTFYoke<'static>, FontData>::attach_to_cart(data.clone(), |data| {
                let font_ref = FontRef::from_index(data.as_ref(), index).unwrap();

                let hinting_instance = HintingInstance::new(
                    &font_ref.outline_glyphs(),
                    Size::new(UNITS_PER_EM),
                    LocationRef::default(),
                    HintingOptions::default(),
                    InterpreterVersion::_35,
                )
                .ok();

                OTFYoke {
                    font_ref: font_ref.clone(),
                    outline_glyphs: font_ref.outline_glyphs(),
                    hinting_instance,
                    glyph_metrics: font_ref
                        .glyph_metrics(Size::new(UNITS_PER_EM), LocationRef::default()),
                }
            });

        Some(Self(Arc::new(font_ref_yoke)))
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

        let draw_settings = if let Some(instance) = self.0.get().hinting_instance.as_ref() {
            // Note: We always hint at the font size `UNITS_PER_EM`, which obviously isn't very useful. We don't do this
            // for better text quality (right now), but instead because there are some PDFs with obscure fonts that
            // actually render wrongly if hinting is disabled!
            // Adding proper hinting support is definitely planned for the future, though.
            DrawSettings::hinted(instance, false)
        } else {
            DrawSettings::unhinted(Size::new(UNITS_PER_EM), LocationRef::default())
        };

        let Some(outline) = self.outline_glyphs().get(glyph) else {
            return BezPath::new();
        };

        let _ = outline.draw(draw_settings, &mut path);
        path.0
    }

    pub fn num_glyphs(&self) -> u16 {
        self.font_ref().maxp().map(|m| m.num_glyphs()).unwrap_or(0)
    }
}

fn convert_matrix(matrix: Matrix) -> Affine {
    Affine::new([
        matrix.sx as f64,
        matrix.kx as f64,
        matrix.ky as f64,
        matrix.sy as f64,
        matrix.tx as f64,
        matrix.ty as f64,
    ])
}

#[derive(Yokeable, Clone)]
struct OTFYoke<'a> {
    pub font_ref: FontRef<'a>,
    pub glyph_metrics: GlyphMetrics<'a>,
    pub hinting_instance: Option<HintingInstance>,
    pub outline_glyphs: OutlineGlyphCollection<'a>,
}

#[derive(Yokeable, Clone)]
struct CFFYoke<'a> {
    pub table: cff::Table<'a>,
}

#[derive(Yokeable, Clone)]
struct Type1Yoke<'a> {
    pub table: type1::Table<'a>,
}
