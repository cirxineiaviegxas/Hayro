use crate::font::Encoding;
use crate::font::blob::OpenTypeFontBlob;
use crate::font::encoding::{GLYPH_NAMES, MAC_OS_ROMAN_INVERSE, MAC_ROMAN_INVERSE};
use crate::font::standard::{StandardFont, select_standard_font};
use crate::util::{CodeMapExt, OptionLog};
use bitflags::bitflags;
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{
    BASE_ENCODING, BASE_FONT, DIFFERENCES, ENCODING, FIRST_CHAR, FLAGS, FONT_DESC, FONT_FILE2,
    LAST_CHAR, MISSING_WIDTH, WIDTHS,
};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::name::names::*;
use hayro_syntax::object::stream::Stream;
use kurbo::BezPath;
use log::warn;
use skrifa::raw::TableProvider;
use skrifa::raw::tables::cmap::PlatformId;
use skrifa::{GlyphId, GlyphId16};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
enum OpenTypeFont {
    Standard(StandardFont),
    Custom(OpenTypeFontBlob),
}

impl OpenTypeFont {
    fn blob(&self) -> OpenTypeFontBlob {
        match self {
            OpenTypeFont::Standard(s) => s.get_blob().clone(),
            OpenTypeFont::Custom(c) => c.clone(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct TrueTypeFont {
    base_font: OpenTypeFont,
    widths: Vec<f32>,
    font_flags: FontFlags,
    encoding: Encoding,
    cached_mappings: RefCell<HashMap<u8, GlyphId>>,
}

impl TrueTypeFont {
    pub fn new(dict: &Dict) -> Option<TrueTypeFont> {
        let descriptor = dict.get::<Dict>(FONT_DESC).unwrap_or_default();

        let font_flags = descriptor
            .get::<u32>(FLAGS)
            .and_then(|n| FontFlags::from_bits(n))
            .unwrap_or(FontFlags::empty());

        let widths = read_widths(dict, &descriptor);
        let (encoding, _) = read_encoding(dict);
        let base_font = select_standard_font(dict)
            .map(|d| OpenTypeFont::Standard(d))
            .or_else(|| {
                descriptor
                    .get::<Stream>(FONT_FILE2)
                    .and_then(|s| s.decoded().ok())
                    .and_then(|d| {
                        OpenTypeFontBlob::new(Arc::new(d.to_vec()), 0)
                            .map(|b| OpenTypeFont::Custom(b))
                    })
            })
            .unwrap_or_else(|| {
                warn!(
                    "failed to extract base font {:?}. falling back to Times New Roman.",
                    dict.get::<Name>(BASE_FONT).map(|b| b.as_str().to_string())
                );

                OpenTypeFont::Standard(StandardFont::TimesRoman)
            });

        Some(Self {
            base_font,
            widths,
            font_flags,
            encoding,
            cached_mappings: RefCell::new(HashMap::new()),
        })
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        match &self.base_font {
            OpenTypeFont::Standard(s) => s.get_blob().outline_glyph(glyph),
            OpenTypeFont::Custom(c) => c.outline_glyph(glyph),
        }
    }

    // TODO: Cache this
    pub fn map_code(&self, code: u8) -> GlyphId {
        if let Some(glyph) = self.cached_mappings.borrow().get(&code) {
            return *glyph;
        }

        let mut glyph = None;

        if self.font_flags.contains(FontFlags::NON_SYMBOLIC)
            && matches!(self.encoding, Encoding::MacRoman | Encoding::WinAnsi)
        {
            let Some(lookup) = self.encoding.lookup(code) else {
                return GlyphId::NOTDEF;
            };

            if let Ok(cmap) = self.base_font.blob().font_ref().cmap() {
                for record in cmap.encoding_records() {
                    if record.platform_id() == PlatformId::Windows && record.encoding_id() == 1 {
                        if let Ok(subtable) = record.subtable(cmap.offset_data()) {
                            glyph = glyph.or_else(|| {
                                GLYPH_NAMES
                                    .get(lookup)
                                    .and_then(|n| n.chars().next())
                                    .and_then(|c| subtable.map_codepoint(c))
                                    .filter(|g| *g != GlyphId::NOTDEF)
                            })
                        }
                    }
                }

                for record in cmap.encoding_records() {
                    if record.platform_id() == PlatformId::Macintosh && record.encoding_id() == 0 {
                        if let Ok(subtable) = record.subtable(cmap.offset_data()) {
                            glyph = glyph.or_else(|| {
                                MAC_OS_ROMAN_INVERSE
                                    .get(lookup)
                                    .or_else(|| MAC_ROMAN_INVERSE.get(lookup))
                                    .and_then(|c| subtable.map_codepoint(*c))
                                    .filter(|g| *g != GlyphId::NOTDEF)
                            })
                        }
                    }
                }
            }

            if glyph.is_none() {
                if let Ok(post) = self.base_font.blob().font_ref().post() {
                    for i in 0..self.base_font.blob().num_glyphs() {
                        if post.glyph_name(GlyphId16::new(i)) == Some(lookup) {
                            glyph = Some(GlyphId::new(i as u32));
                        }
                    }
                }
            }
        } else {
            if let Ok(cmap) = self.base_font.blob().font_ref().cmap() {
                for record in cmap.encoding_records() {
                    if record.platform_id() == PlatformId::Windows && record.encoding_id() == 0 {
                        if let Ok(subtable) = record.subtable(cmap.offset_data()) {
                            for offset in [0x0000u32, 0xF000, 0xF100, 0xF200] {
                                glyph = glyph
                                    .or_else(|| subtable.map_codepoint(code as u32 + offset))
                                    .filter(|g| *g != GlyphId::NOTDEF)
                            }
                        }
                    } else if record.platform_id() == PlatformId::Macintosh
                        && record.encoding_id() == 0
                    {
                        if let Ok(subtable) = record.subtable(cmap.offset_data()) {
                            glyph = glyph
                                .or_else(|| subtable.map_codepoint(code))
                                .filter(|g| *g != GlyphId::NOTDEF)
                        }
                    }
                }
            }
        }

        let glyph = glyph.unwrap_or(GlyphId::NOTDEF);
        self.cached_mappings.borrow_mut().insert(code, glyph);

        glyph
    }

    pub fn glyph_width(&self, code: u8) -> f32 {
        self.widths
            .get(code as usize)
            .copied()
            .or_else(|| {
                self.base_font
                    .blob()
                    .glyph_metrics()
                    .advance_width(self.map_code(code))
            })
            .warn_none(&format!("failed to find advance width for code {code}"))
            .unwrap_or(0.0)
    }
}

bitflags! {
    /// Bitflags describing various characteristics of fonts.
    #[derive(Debug)]
    pub struct FontFlags: u32 {
        const FIXED_PITCH = 1 << 0;
        const SERIF = 1 << 1;
        const SYMBOLIC = 1 << 2;
        const SCRIPT = 1 << 3;
        const NON_SYMBOLIC = 1 << 5;
        const ITALIC = 1 << 6;
        const ALL_CAP = 1 << 16;
        const SMALL_CAP = 1 << 17;
        const FORCE_BOLD = 1 << 18;
    }
}

pub(crate) fn read_widths(dict: &Dict, descriptor: &Dict) -> Vec<f32> {
    let mut widths = Vec::new();

    let first_char = dict.get::<usize>(FIRST_CHAR);
    let last_char = dict.get::<usize>(LAST_CHAR);
    let widths_arr = dict.get::<Array>(WIDTHS);
    let missing_width = descriptor.get::<f32>(MISSING_WIDTH).unwrap_or(0.0);

    match (first_char, last_char, widths_arr) {
        (Some(fc), Some(lc), Some(w)) => {
            let mut iter = w.iter::<f32>().take(lc - fc + 1);

            for _ in 0..fc {
                widths.push(missing_width);
            }

            while let Some(w) = iter.next() {
                widths.push(w);
            }

            while widths.len() <= (u8::MAX as usize) + 1 {
                widths.push(missing_width);
            }
        }
        _ => {}
    }

    widths
}

pub(crate) fn read_encoding(dict: &Dict) -> (Encoding, HashMap<u8, String>) {
    fn get_encoding_base(dict: &Dict, name: &Name) -> Encoding {
        match dict.get::<Name>(name) {
            Some(n) => match n.as_ref() {
                WIN_ANSI_ENCODING => Encoding::WinAnsi,
                MAC_ROMAN_ENCODING => Encoding::MacRoman,
                MAC_EXPERT_ENCODING => Encoding::MacExpert,
                _ => {
                    warn!("Unknown font encoding {}", name.as_str());

                    Encoding::Standard
                }
            },
            None => Encoding::BuiltIn,
        }
    }

    let mut map = HashMap::new();

    if let Some(encoding_dict) = dict.get::<Dict>(ENCODING) {
        // Note that those only exist for Type1 and Type3 fonts, not for TrueType fonts.
        if let Some(differences) = encoding_dict.get::<Array>(DIFFERENCES) {
            let mut entries = differences.iter::<Object>();

            let mut code = 0;

            while let Some(obj) = entries.next() {
                if let Ok(num) = obj.clone().cast::<i32>() {
                    code = num;
                } else if let Ok(name) = obj.cast::<Name>() {
                    map.insert(code as u8, name.as_str().to_string());
                    code += 1;
                }
            }
        }

        (get_encoding_base(&encoding_dict, BASE_ENCODING), map)
    } else {
        (get_encoding_base(&dict, ENCODING), HashMap::new())
    }
}
