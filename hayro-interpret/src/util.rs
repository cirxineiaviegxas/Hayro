use log::warn;
use skrifa::GlyphId;
use skrifa::raw::tables::cmap::CmapSubtable;

pub(crate) trait OptionLog {
    fn warn_none(self, f: &str) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn warn_none(self, f: &str) -> Self {
        self.or_else(|| {
            warn!("{}", f);

            None
        })
    }
}

pub(crate) trait CodeMapExt {
    fn map_codepoint(&self, code: impl Into<u32>) -> Option<GlyphId>;
}

impl CodeMapExt for CmapSubtable<'_> {
    fn map_codepoint(&self, code: impl Into<u32>) -> Option<GlyphId> {
        match self {
            CmapSubtable::Format0(f) => f.map_codepoint(code),
            CmapSubtable::Format4(f) => f.map_codepoint(code),
            CmapSubtable::Format6(f) => f.map_codepoint(code),
            CmapSubtable::Format12(f) => f.map_codepoint(code),
            _ => {
                warn!("unsupported cmap table {:?}", self);

                None
            }
        }
    }
}
