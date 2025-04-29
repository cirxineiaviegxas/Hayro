use crate::font::encodings::{COURIER, HELVETICA, SYMBOL, TIMES_ROMAN, ZAPF_DING_BATS};
use crate::font::glyph_list::{GLYPH_NAMES, ZAPF_DINGS};
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
            Self::Helvetica => HELVETICA.get(&code),
            Self::HelveticaBold => HELVETICA.get(&code),
            Self::HelveticaOblique => HELVETICA.get(&code),
            Self::HelveticaBoldOblique => HELVETICA.get(&code),
            Self::Courier => COURIER.get(&code),
            Self::CourierBold => COURIER.get(&code),
            Self::CourierOblique => COURIER.get(&code),
            Self::CourierBoldOblique => COURIER.get(&code),
            Self::TimesRoman => TIMES_ROMAN.get(&code),
            Self::TimesBold => TIMES_ROMAN.get(&code),
            Self::TimesItalic => TIMES_ROMAN.get(&code),
            Self::TimesBoldItalic => TIMES_ROMAN.get(&code),
            Self::Symbol => SYMBOL.get(&code),
            // Note that this font does not return postscript character names,
            // but instead has a custom encoding.
            Self::ZapfDingBats => ZAPF_DING_BATS.get(&code),
        }
        .copied()
    }

    pub fn ps_to_unicode(&self, name: &str) -> Option<&'static str> {
        match self {
            Self::ZapfDingBats => ZAPF_DINGS.get(name),
            _ => GLYPH_NAMES.get(name),
        }
        .warn_none(&format!("failed to map code {name} for {:?}", self))
        .copied()
    }

    pub fn map_code(&self, code: u8) -> Option<&'static str> {
        self.ps_to_unicode(self.code_to_name(code)?)
    }
}

#[cfg(test)]
mod tests {

    // TODO: Check whether fallback fonts cover all chars of standard fonts
}
