use crate::font::blob::{CffFontBlob, OpenTypeFontBlob};
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{
    CID_TO_GID_MAP, DESCENDANT_FONTS, DW, ENCODING, FONT_DESCRIPTOR, FONT_FILE2, FONT_FILE3,
    SUBTYPE, W,
};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::name::names::{
    CID_FONT_TYPE_0C, IDENTITY, IDENTITY_H, IDENTITY_V, OPEN_TYPE,
};
use hayro_syntax::object::stream::Stream;
use log::warn;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId};
use std::collections::HashMap;
use std::sync::Arc;
use kurbo::BezPath;

#[derive(Debug)]
pub(crate) struct Type0Font {
    font_type: FontType,
    dw: f32,
    widths: HashMap<u16, f32>,
    cid_to_gid_map: CidToGIdMap,
}

impl Type0Font {
    pub fn new(dict: &Dict) -> Option<Self> {
        if !dict
            .get::<Name>(ENCODING)
            .is_some_and(|n| matches!(n.as_ref(), IDENTITY_H | IDENTITY_V))
        {
            warn!("CID fonts with custom encoding are currently unsupported");

            return None;
        }

        let descendant_font = dict.get::<Array>(DESCENDANT_FONTS)?.iter::<Dict>().next()?;
        let font_descriptor = descendant_font.get::<Dict>(FONT_DESCRIPTOR)?;
        let font_type = FontType::new(&font_descriptor)?;

        let default_width = dict.get::<f32>(DW).unwrap_or(1000.0);
        let widths = dict
            .get::<Array>(W)
            .and_then(|a| read_widths(&a))
            .unwrap_or_default();
        let cid_to_gid_map = CidToGIdMap::new(dict).unwrap_or_default();

        Some(Self {
            font_type,
            dw: default_width,
            widths,
            cid_to_gid_map,
        })
    }

    pub fn map_code(&self, code: u16) -> GlyphId {
        match &self.font_type {
            FontType::TrueType(_) => {
                self.cid_to_gid_map.map(code)
            }
            FontType::Cff(c) => {
                let table = c.table();
                
                if table.is_cid() {
                    table.glyph_index_by_cid(code).map(|g| GlyphId::new(g.0 as u32))
                        .unwrap_or(GlyphId::NOTDEF)
                }   else {
                    GlyphId::new(code as u32)
                }
            }
        }
    }

    pub fn outline_glyph(&self, glyph: GlyphId) -> BezPath {
        match &self.font_type {
            FontType::TrueType(t) => t.outline_glyph(glyph),
            FontType::Cff(c) => c.outline_glyph(glyph),
        }
    }

    pub fn code_width(&self, code: u16) -> f32 {
        self.widths.get(&code).copied().unwrap_or(self.dw)
    }

    pub fn code_len(&self) -> usize {
        2
    }
}

#[derive(Debug)]
enum FontType {
    /// Type2 CID font.
    TrueType(OpenTypeFontBlob),
    /// Type0 CID font, backed by CFF font program (either via CIDFontType0C or OpenType).
    Cff(CffFontBlob),
}

impl FontType {
    fn new(descriptor: &Dict) -> Option<Self> {
        // Apparently there are some PDFs that have the wrong subtype,
        // so we just brute-force trying to parse the correct type to give
        // some leeway.

        if let Some(stream) = descriptor.get::<Stream>(FONT_FILE2) {
            let decoded = stream.decoded().ok()?;
            let data = Arc::new(decoded.to_vec());

            return Some(Self::TrueType(OpenTypeFontBlob::new(data, 0)?));
        } else if let Some(stream) = descriptor.get::<Stream>(FONT_FILE3) {
            let decoded = stream.decoded().ok()?;

            return match stream.dict().get::<Name>(SUBTYPE)?.as_ref() {
                CID_FONT_TYPE_0C => {
                    let data = Arc::new(decoded.to_vec());

                    Some(Self::Cff(CffFontBlob::new(data)?))
                }
                OPEN_TYPE => {
                    let font_ref = FontRef::new(decoded.as_ref()).ok()?;
                    let cff_data = Arc::new(font_ref.cff().ok()?.offset_data().as_ref().to_vec());

                    Some(Self::Cff(CffFontBlob::new(cff_data)?))
                }
                _ => {
                    warn!("unknown subtype for FontFile3");

                    None
                }
            };
        }

        warn!("CID font didn't have an embededd font file");

        None
    }

    fn is_type0(&self) -> bool {
        !matches!(self, FontType::TrueType(_))
    }
}

#[derive(Debug, Default)]
enum CidToGIdMap {
    #[default]
    Identity,
    Mapped(HashMap<u16, GlyphId>),
}

impl CidToGIdMap {
    pub fn new(dict: &Dict) -> Option<Self> {
        if let Some(name) = dict.get::<Name>(CID_TO_GID_MAP) {
            if name.as_ref() == IDENTITY {
                Some(CidToGIdMap::Identity)
            } else {
                None
            }
        } else if let Some(stream) = dict.get::<Stream>(CID_TO_GID_MAP) {
            let decoded = stream.decoded().ok()?;
            let mut map = HashMap::new();

            for (cid, gid) in decoded.chunks_exact(2).enumerate() {
                let gid = u16::from_be_bytes([gid[0], gid[1]]);

                map.insert(cid as u16, GlyphId::new(gid as u32));
            }

            Some(CidToGIdMap::Mapped(map))
        } else {
            None
        }
    }

    pub fn map(&self, code: u16) -> GlyphId {
        match self {
            CidToGIdMap::Identity => GlyphId::new(code as u32),
            CidToGIdMap::Mapped(map) => map.get(&code).copied().unwrap_or(GlyphId::NOTDEF),
        }
    }
}

fn read_widths(arr: &Array) -> Option<HashMap<u16, f32>> {
    let mut map = HashMap::new();
    let mut iter = arr.iter::<Object>();

    while let Some(mut first) = iter.next().and_then(|o| o.cast::<u16>().ok()) {
        let second = iter.next()?;

        if let Some(second) = second.clone().cast::<u16>().ok() {
            let width = iter.next().and_then(|o| o.cast::<f32>().ok())?;

            for i in first..=second {
                map.insert(i, width);
            }
        } else if let Some(range) = second.cast::<Array>().ok() {
            for width in range.iter::<f32>() {
                map.insert(first, width);
                first.checked_add(1)?;
            }
        }
    }

    Some(map)
}
