use crate::font::encoding::{GLYPH_NAMES, ZAPF_DINGS_NAMES};
use crate::font::encoding::{STANDARD, SYMBOL, ZAPF_DING_BATS};
use crate::util::OptionLog;

#[derive(Copy, Clone, Debug)]
pub(crate) enum BaseFont {
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

impl BaseFont {
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

    pub fn ps_to_unicode(&self, name: &str) -> Option<&'static str> {
        match self {
            Self::ZapfDingBats => ZAPF_DINGS_NAMES.get(name),
            _ => GLYPH_NAMES.get(name),
        }
        .warn_none(&format!("failed to map code {name} for {:?}", self))
        .copied()
    }

    pub fn map_code(&self, code: u8) -> Option<&'static str> {
        self.ps_to_unicode(self.code_to_name(code)?)
    }
}
