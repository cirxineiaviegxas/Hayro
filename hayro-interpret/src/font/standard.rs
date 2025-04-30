use crate::font::blob;
use crate::font::blob::{
    COURIER_BOLD, COURIER_BOLD_ITALIC, COURIER_ITALIC, COURIER_REGULAR, FontBlob, HELVETICA_BOLD,
    HELVETICA_BOLD_ITALIC, HELVETICA_ITALIC, HELVETICA_REGULAR, TIMES_BOLD, TIMES_ITALIC,
    TIMES_REGULAR, TIMES_ROMAN_BOLD_ITALIC, ZAPF_DINGS_BAT,
};
use crate::font::encoding::{GLYPH_NAMES, ZAPF_DINGS_NAMES};
use crate::font::encoding::{STANDARD, SYMBOL, ZAPF_DING_BATS};
use crate::util::OptionLog;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::BASE_FONT;
use hayro_syntax::object::name::Name;
use log::warn;

#[derive(Copy, Clone, Debug)]
pub(crate) enum StandardFont {
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,
    Courier,
    CourierBold,
    CourierOblique,
    CourierBoldOblique,
    TimesRoman,
    TimesBold,
    TimesItalic,
    TimesBoldItalic,
    ZapfDingBats,
    Symbol,
}

impl StandardFont {
    pub fn code_to_name(&self, code: u8) -> Option<&'static str> {
        match self {
            Self::Symbol => SYMBOL.get(&code),
            // Note that this font does not return postscript character names,
            // but instead has a custom encoding.
            Self::ZapfDingBats => ZAPF_DING_BATS.get(&code),
            _ => STANDARD.get(&code),
        }
        .copied()
    }

    pub fn name_to_unicode(&self, name: &str) -> Option<&'static str> {
        match self {
            Self::ZapfDingBats => ZAPF_DINGS_NAMES.get(name),
            _ => GLYPH_NAMES.get(name),
        }
        .warn_none(&format!("failed to map code {name} for {:?}", self))
        .copied()
    }

    pub fn code_to_unicode(&self, code: u8) -> Option<&'static str> {
        self.name_to_unicode(self.code_to_name(code)?)
    }

    pub fn get_blob(&self) -> FontBlob {
        match self {
            StandardFont::Helvetica => HELVETICA_REGULAR.clone(),
            StandardFont::HelveticaBold => HELVETICA_BOLD.clone(),
            StandardFont::HelveticaOblique => HELVETICA_ITALIC.clone(),
            StandardFont::HelveticaBoldOblique => HELVETICA_BOLD_ITALIC.clone(),
            StandardFont::Courier => COURIER_REGULAR.clone(),
            StandardFont::CourierBold => COURIER_BOLD.clone(),
            StandardFont::CourierOblique => COURIER_ITALIC.clone(),
            StandardFont::CourierBoldOblique => COURIER_BOLD_ITALIC.clone(),
            StandardFont::TimesRoman => TIMES_REGULAR.clone(),
            StandardFont::TimesBold => TIMES_BOLD.clone(),
            StandardFont::TimesItalic => TIMES_ITALIC.clone(),
            StandardFont::TimesBoldItalic => TIMES_ROMAN_BOLD_ITALIC.clone(),
            StandardFont::ZapfDingBats => ZAPF_DINGS_BAT.clone(),
            StandardFont::Symbol => blob::SYMBOL.clone(),
        }
    }
}

pub(crate) fn select_standard_font(dict: &Dict) -> Option<StandardFont> {
    // See <https://github.com/apache/pdfbox/blob/4438b8fdc67a3a9ebfb194595d0e81f88b708a37/pdfbox/src/main/java/org/apache/pdfbox/pdmodel/font/FontMapperImpl.java#L62-L102>
    match dict.get::<Name>(BASE_FONT)?.get().as_ref() {
        b"Helvetica" | b"ArialMT" | b"Arial" | b"LiberationSans" | b"NimbusSanL-Regu" => {
            Some(StandardFont::Helvetica)
        }
        b"Helvetica-Bold"
        | b"Arial-BoldMT"
        | b"Arial-Bold"
        | b"LiberationSans-Bold"
        | b"NimbusSanL-Bold" => Some(StandardFont::HelveticaBold),
        b"Helvetica-Oblique"
        | b"Arial-ItalicMT"
        | b"Arial-Italic"
        | b"Helvetica-Italic"
        | b"LiberationSans-Italic"
        | b"NimbusSanL-ReguItal" => Some(StandardFont::HelveticaOblique),
        b"Helvetica-BoldOblique"
        | b"Arial-BoldItalicMT"
        | b"Helvetica-BoldItalic"
        | b"LiberationSans-BoldItalic"
        | b"NimbusSanL-BoldItal" => Some(StandardFont::HelveticaBoldOblique),
        b"Courier" | b"CourierNew" | b"CourierNewPSMT" | b"LiberationMono" | b"NimbusMonL-Regu" => {
            Some(StandardFont::Courier)
        }
        b"Courier-Bold"
        | b"CourierNewPS-BoldMT"
        | b"CourierNew-Bold"
        | b"LiberationMono-Bold"
        | b"NimbusMonL-Bold" => Some(StandardFont::CourierBold),
        b"Courier-Oblique"
        | b"CourierNewPS-ItalicMT"
        | b"CourierNew-Italic"
        | b"LiberationMono-Italic"
        | b"NimbusMonL-ReguObli" => Some(StandardFont::CourierOblique),
        b"Courier-BoldOblique"
        | b"CourierNewPS-BoldItalicMT"
        | b"CourierNew-BoldItalic"
        | b"LiberationMono-BoldItalic"
        | b"NimbusMonL-BoldObli" => Some(StandardFont::CourierBoldOblique),
        b"Times-Roman"
        | b"TimesNewRomanPSMT"
        | b"TimesNewRoman"
        | b"TimesNewRomanPS"
        | b"LiberationSerif"
        | b"NimbusRomNo9L-Regu" => Some(StandardFont::TimesRoman),
        b"Times-Bold"
        | b"TimesNewRomanPS-BoldMT"
        | b"TimesNewRomanPS-Bold"
        | b"TimesNewRoman-Bold"
        | b"LiberationSerif-Bold"
        | b"NimbusRomNo9L-Medi" => Some(StandardFont::TimesBold),
        b"Times-Italic"
        | b"TimesNewRomanPS-ItalicMT"
        | b"TimesNewRomanPS-Italic"
        | b"TimesNewRoman-Italic"
        | b"LiberationSerif-Italic"
        | b"NimbusRomNo9L-ReguItal" => Some(StandardFont::TimesItalic),
        b"Times-BoldItalic"
        | b"TimesNewRomanPS-BoldItalicMT"
        | b"TimesNewRomanPS-BoldItalic"
        | b"TimesNewRoman-BoldItalic"
        | b"LiberationSerif-BoldItalic"
        | b"NimbusRomNo9L-MediItal" => Some(StandardFont::TimesBoldItalic),
        b"Symbol" | b"SymbolMT" | b"StandardSymL" => Some(StandardFont::Symbol),
        b"ZapfDingbats"
        | b"ZapfDingbatsITCbyBT-Regular"
        | b"ZapfDingbatsITC"
        | b"Dingbats"
        | b"MS-Gothic" => Some(StandardFont::ZapfDingBats),
        _ => None,
    }
}
