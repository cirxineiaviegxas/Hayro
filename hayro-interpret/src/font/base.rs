use crate::font::encodings::HELVETICA;
use crate::font::glyph_list::GLYPH_NAMES;
use crate::util::OptionLog;

pub(crate) enum BaseFont {
    Helvetica,
}

impl BaseFont {
    pub fn map_code(&self, code: u8) -> Option<&'static str> {
        let ps_name = match self {
            Self::Helvetica => HELVETICA.get(&code),
        };

        ps_name
            .and_then(|name| GLYPH_NAMES.get(name))
            .warn_none(&format!("failed to map code {code} for Helvetica"))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use crate::font::encodings::HELVETICA;
    use crate::font::glyph_list::GLYPH_NAMES;
    use skrifa::{FontRef, MetadataProvider};

    // TODO: Check whether fallback fonts cover all chars of standard fonts
}
