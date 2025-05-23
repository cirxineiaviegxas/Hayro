use crate::file::xref::XRef;
use crate::trivia::{Comment, is_eol_character, is_white_space_character};
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    #[inline]
    pub fn at_end(&self) -> bool {
        self.offset >= self.data.len()
    }

    #[inline]
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    #[inline]
    pub fn jump_to_end(&mut self) {
        self.offset = self.data.len();
    }

    #[inline]
    pub fn jump(&mut self, offset: usize) {
        self.offset = offset;
    }

    #[inline]
    pub fn tail(&mut self) -> Option<&'a [u8]> {
        self.data.get(self.offset..)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn range(&self, range: Range<usize>) -> Option<&'a [u8]> {
        self.data.get(range)
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    #[inline]
    pub fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        let v = self.peek_bytes(len)?;
        self.offset += len;

        Some(v)
    }

    #[inline]
    pub fn read_byte(&mut self) -> Option<u8> {
        let v = self.peek_byte()?;
        self.offset += 1;

        Some(v)
    }

    #[inline]
    pub(crate) fn read<const PLAIN: bool, T: Readable<'a>>(
        &mut self,
        xref: &XRef<'a>,
    ) -> Option<T> {
        let old_offset = self.offset;

        T::read::<PLAIN>(self, &xref).or_else(|| {
            self.offset = old_offset;

            None
        })
    }

    #[inline]
    pub fn read_with_xref<T: Readable<'a>>(&mut self, xref: &XRef<'a>) -> Option<T> {
        self.read::<false, T>(xref)
    }

    #[inline]
    pub fn read_without_xref<T: Readable<'a>>(&mut self) -> Option<T> {
        self.read::<true, T>(&XRef::dummy())
    }

    #[inline]
    pub(crate) fn skip<const PLAIN: bool, T: Skippable>(&mut self) -> Option<&'a [u8]> {
        let old_offset = self.offset;

        T::skip::<PLAIN>(self).or_else(|| {
            self.offset = old_offset;
            None
        })?;

        self.data.get(old_offset..self.offset)
    }

    #[inline]
    pub fn skip_non_plain<T: Skippable>(&mut self) -> Option<&'a [u8]> {
        self.skip::<false, T>()
    }

    #[inline]
    pub fn skip_plain<T: Skippable>(&mut self) -> Option<&'a [u8]> {
        self.skip::<true, T>()
    }

    #[inline]
    pub fn skip_bytes(&mut self, len: usize) -> Option<()> {
        self.read_bytes(len).map(|_| {})
    }

    #[inline]
    pub fn peek_bytes(&self, len: usize) -> Option<&'a [u8]> {
        self.data.get(self.offset..self.offset + len)
    }

    #[inline]
    pub fn peek_byte(&self) -> Option<u8> {
        self.data.get(self.offset).copied()
    }

    #[inline]
    pub fn eat(&mut self, f: impl Fn(u8) -> bool) -> Option<u8> {
        let val = self.peek_byte()?;
        if f(val) {
            self.forward();
            Some(val)
        } else {
            None
        }
    }

    #[inline]
    pub fn forward(&mut self) {
        self.offset += 1;
    }

    #[inline]
    pub fn forward_if(&mut self, f: impl Fn(u8) -> bool) -> Option<()> {
        if f(self.peek_byte()?) {
            self.forward();

            Some(())
        } else {
            None
        }
    }

    #[inline]
    pub fn forward_while_1(&mut self, f: impl Fn(u8) -> bool) -> Option<()> {
        self.eat(&f)?;
        self.forward_while(f);
        Some(())
    }

    #[inline]
    pub fn forward_tag(&mut self, tag: &[u8]) -> Option<()> {
        self.peek_tag(tag)?;
        self.offset += tag.len();

        Some(())
    }

    #[inline]
    pub fn forward_while(&mut self, f: impl Fn(u8) -> bool) {
        while let Some(b) = self.peek_byte() {
            if f(b) {
                self.forward();
            } else {
                break;
            }
        }
    }

    #[inline]
    pub fn peek_tag(&self, tag: &[u8]) -> Option<()> {
        let mut cloned = self.clone();

        for b in tag.iter().copied() {
            if cloned.peek_byte() == Some(b) {
                cloned.forward();
            } else {
                return None;
            }
        }

        Some(())
    }

    #[inline]
    pub fn skip_white_spaces(&mut self) {
        while let Some(b) = self.peek_byte() {
            if is_white_space_character(b) {
                self.forward();
            } else {
                return;
            }
        }
    }

    #[inline]
    pub fn skip_eol_characters(&mut self) {
        while let Some(b) = self.peek_byte() {
            if is_eol_character(b) {
                self.forward();
            } else {
                return;
            }
        }
    }

    #[inline]
    pub fn skip_white_spaces_and_comments(&mut self) {
        while let Some(b) = self.peek_byte() {
            if is_white_space_character(b) {
                self.skip_white_spaces()
            } else if b == b'%' {
                Comment::skip::<true>(self);
            } else {
                return;
            }
        }
    }
}

pub trait Readable<'a>: Sized {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self>;
    fn from_bytes(b: &'a [u8]) -> Option<Self> {
        let mut r = Reader::new(b);
        let xref = XRef::dummy();

        Self::read::<false>(&mut r, &xref)
    }
}

pub trait Skippable {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()>;
}
