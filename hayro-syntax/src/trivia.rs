use crate::file::xref::XRef;
use crate::reader::{Readable, Reader, Skippable};

#[inline(always)]
pub fn is_white_space_character(char: u8) -> bool {
    match char {
        0x00 | 0x09 | 0x0a | 0x0c | 0x0d | 0x20 => true,
        _ => false,
    }
}

#[inline(always)]
pub fn is_regular_character(char: u8) -> bool {
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
pub fn is_eol_character(char: u8) -> bool {
    match char {
        0x0a | 0x0d => true,
        _ => false,
    }
}

#[inline(always)]
pub fn is_delimiter_character(char: u8) -> bool {
    match char {
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' => true,
        _ => false,
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Comment<'a>(pub &'a [u8]);

impl Skippable for Comment<'_> {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.forward_tag(b"%")?;
        r.forward_while(|b| !is_eol_character(b));

        Some(())
    }
}

impl<'a> Readable<'a> for Comment<'a> {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, _: &XRef<'a>) -> Option<Self> {
        let bytes = r.skip_plain::<Comment>()?;
        let bytes = bytes.get(1..bytes.len()).unwrap();

        Some(Comment(bytes))
    }
}
