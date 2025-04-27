use crate::font::encodings::HELVETICA;
use crate::font::glyph_list::GLYPH_NAMES;
use crate::util::OptionLog;

#[derive(Copy, Clone, Debug)]
pub(crate) enum BaseFont {
    Helvetica,
}

impl BaseFont {
    pub fn code_to_ps(&self, code: u8) -> Option<&'static str> {
        match self {
            Self::Helvetica => HELVETICA.get(&code),
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
