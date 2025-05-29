//! Comments and white spaces.

use crate::reader::{Readable, Reader, Skippable};
use crate::xref::XRef;

#[inline(always)]
pub(crate) fn is_white_space_character(char: u8) -> bool {
    match char {
        0x00 | 0x09 | 0x0a | 0x0c | 0x0d | 0x20 => true,
        _ => false,
    }
}

#[inline(always)]
pub(crate) fn is_regular_character(char: u8) -> bool {
    match char {
        // Whitespace characters
        0x00 | 0x09 | 0x0a | 0x0c | 0x0d | 0x20 => false,
        // Delimiter characters
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' => false,
        // All other characters are considered regular.
        _ => true,
    }
}

#[inline(always)]
pub(crate) fn is_eol_character(char: u8) -> bool {
    match char {
        0x0a | 0x0d => true,
        _ => false,
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub(crate) struct Comment<'a>(pub &'a [u8]);

impl Skippable for Comment<'_> {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.forward_tag(b"%")?;
        r.forward_while(|b| !is_eol_character(b));

        Some(())
    }
}

impl<'a> Readable<'a> for Comment<'a> {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, _: &'a XRef) -> Option<Self> {
        let bytes = r.skip_plain::<Comment>()?;
        let bytes = bytes.get(1..bytes.len()).unwrap();

        Some(Comment(bytes))
    }
}
