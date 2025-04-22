use crate::file::xref::XRef;
use crate::object;
use crate::object::Object;
use crate::reader::{Readable, Reader, Skippable};
use crate::trivia::is_regular_character;
use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// A PDF name.
#[derive(Debug, Eq, Clone, Copy)]
pub struct Name<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) has_escape: bool,
}

// Custom PartialEq and Hash implementation.
// We do this so that when having a dict where the key is a name, escaped and unescaped
// versions of the same name get mapped to the same key.
impl PartialEq for Name<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Hash for Name<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<'a> Name<'a> {
    /// Create a new name from an unescaped sequence of bytes.
    pub const fn from_unescaped(data: &'a [u8]) -> Name<'a> {
        Self {
            data,
            has_escape: false,
        }
    }

    /// Return a string representation of the name.
    pub fn as_str(&self) -> String {
        std::str::from_utf8(&self.get())
            .unwrap_or("{non-ascii key}")
            .to_string()
    }

    pub(crate) fn get(&self) -> Cow<'a, [u8]> {
        escape_name_like(self.data, self.has_escape)
    }
}

object!(Name<'a>, Name);

// This method is shared by `Name` and the parser for content stream operators (which behave like
// names, except that they aren't preceded by a solidus.
pub(crate) fn escape_name_like(data: &[u8], has_escape: bool) -> Cow<[u8]> {
    fn convert_hex(c: u8) -> u8 {
        match c {
            b'A'..=b'F' => c - b'A' + 10,
            b'a'..=b'f' => c - b'a' + 10,
            b'0'..=b'9' => c - b'0',
            _ => unreachable!(),
        }
    }

    if !has_escape {
        Cow::Borrowed(data)
    } else {
        let mut cleaned = vec![];

        let mut r = Reader::new(data);

        while let Some(b) = r.read_byte() {
            if b == b'#' {
                // We already verified when skipping that it's a valid hex sequence.
                let hex = r.read_bytes(2).unwrap();
                cleaned.push(convert_hex(hex[0]) << 4 | convert_hex(hex[1]));
            } else {
                cleaned.push(b);
            }
        }

        Cow::Owned(cleaned)
    }
}

impl Skippable for Name<'_> {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        skip_name_like(r, true).map(|_| ())
    }
}

impl<'a> Readable<'a> for Name<'a> {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, _: &XRef<'a>) -> Option<Self> {
        let (data, has_escape) = {
            let start = r.offset();
            let has_escape = skip_name_like(r, true)?;
            let end = r.offset();

            let data = r.range(start + 1..end).unwrap();

            (data, has_escape)
        };

        Some(Name { data, has_escape })
    }
}

// This method is shared by `Name` and the parser for content stream operators (which behave like
// names, except that they aren't preceded by a solidus.
pub(crate) fn skip_name_like(r: &mut Reader, solidus: bool) -> Option<bool> {
    let mut has_escape = false;

    if solidus {
        r.forward_tag(b"/")?;
    }

    while let Some(b) = r.eat(|n| is_regular_character(n)) {
        match b {
            b'#' => {
                r.eat(|n| n.is_ascii_hexdigit())?;
                r.eat(|n| n.is_ascii_hexdigit())?;
                has_escape = true;
            }
            _ => {}
        }
    }

    Some(has_escape)
}

#[cfg(test)]
mod tests {
    use crate::object::name::Name;
    use crate::reader::Reader;

    #[test]
    fn name_1() {
        assert_eq!(
            Reader::new("/".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"".to_vec()
        );
    }

    #[test]
    fn name_2() {
        assert!(Reader::new("dfg".as_bytes()).read_plain::<Name>().is_none());
    }

    #[test]
    fn name_3() {
        assert!(
            Reader::new("/AB#FG".as_bytes())
                .read_plain::<Name>()
                .is_none()
        );
    }

    #[test]
    fn name_4() {
        assert_eq!(
            Reader::new("/Name1".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"Name1".to_vec()
        );
    }

    #[test]
    fn name_5() {
        assert_eq!(
            Reader::new("/ASomewhatLongerName".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"ASomewhatLongerName".to_vec()
        );
    }

    #[test]
    fn name_6() {
        assert_eq!(
            Reader::new("/A;Name_With-Various***Characters?".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"A;Name_With-Various***Characters?".to_vec()
        );
    }

    #[test]
    fn name_7() {
        assert_eq!(
            Reader::new("/1.2".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"1.2".to_vec()
        );
    }

    #[test]
    fn name_8() {
        assert_eq!(
            Reader::new("/$$".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"$$".to_vec()
        );
    }

    #[test]
    fn name_9() {
        assert_eq!(
            Reader::new("/@pattern".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"@pattern".to_vec()
        );
    }

    #[test]
    fn name_10() {
        assert_eq!(
            Reader::new("/.notdef".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b".notdef".to_vec()
        );
    }

    #[test]
    fn name_11() {
        assert_eq!(
            Reader::new("/lime#20Green".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"lime Green".to_vec()
        );
    }

    #[test]
    fn name_12() {
        assert_eq!(
            Reader::new("/paired#28#29parentheses".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"paired()parentheses".to_vec()
        );
    }

    #[test]
    fn name_13() {
        assert_eq!(
            Reader::new("/The_Key_of_F#23_Minor".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"The_Key_of_F#_Minor".to_vec()
        );
    }

    #[test]
    fn name_14() {
        assert_eq!(
            Reader::new("/A#42".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"AB".to_vec()
        );
    }

    #[test]
    fn name_15() {
        assert_eq!(
            Reader::new("/A#3b".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"A;".to_vec()
        );
    }

    #[test]
    fn name_16() {
        assert_eq!(
            Reader::new("/A#3B".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"A;".to_vec()
        );
    }

    #[test]
    fn name_17() {
        assert_eq!(
            Reader::new("/k1  ".as_bytes())
                .read_plain::<Name>()
                .unwrap()
                .get(),
            b"k1".to_vec()
        );
    }
}
