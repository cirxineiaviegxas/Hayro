use crate::font::encodings::{COURIER, COURIER_BOLD, COURIER_BOLD_OBLIQUE, COURIER_OBLIQUE, HELVETICA, HELVETICA_BOLD, HELVETICA_BOLD_OBLIQUE, HELVETICA_OBLIQUE, TIMES_BOLD, TIMES_BOLD_ITALIC, TIMES_ITALIC, TIMES_ROMAN};
use crate::font::glyph_list::GLYPH_NAMES;
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
}

impl BaseFont {
    pub fn code_to_ps(&self, code: u8) -> Option<&'static str> {
        match self {
            Self::Helvetica => HELVETICA.get(&code),
            Self::HelveticaBold => HELVETICA_BOLD.get(&code),
            Self::HelveticaOblique => HELVETICA_OBLIQUE.get(&code),
            Self::HelveticaBoldOblique => HELVETICA_BOLD_OBLIQUE.get(&code),
            Self::Courier => COURIER.get(&code),
            Self::CourierBold => COURIER_BOLD.get(&code),
            Self::CourierOblique => COURIER_OBLIQUE.get(&code),
            Self::CourierBoldOblique => COURIER_BOLD_OBLIQUE.get(&code),
            Self::TimesRoman => TIMES_ROMAN.get(&code),
            Self::TimesBold => TIMES_BOLD.get(&code),
            Self::TimesItalic => TIMES_ITALIC.get(&code),
            Self::TimesBoldItalic => TIMES_BOLD_ITALIC.get(&code)
        }
        .copied()
    }

    pub fn ps_to_unicode(&self, name: &str) -> Option<&'static str> {
        GLYPH_NAMES
            .get(name)
            .warn_none(&format!("failed to map code {name} for Helvetica"))
            .copied()
    }

    pub fn map_code(&self, code: u8) -> Option<&'static str> {
        self.ps_to_unicode(self.code_to_ps(code)?)
    }
}

#[cfg(test)]
mod tests {

    // TODO: Check whether fallback fonts cover all chars of standard fonts
}
