use crate::file::xref::XRef;
use crate::object;
use crate::object::Object;
use crate::reader::{Readable, Reader, Skippable};

impl Skippable for bool {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        match r.peek_byte()? {
            b't' => r.forward_tag(b"true"),
            b'f' => r.forward_tag(b"false"),
            _ => None,
        }
    }
}

impl Readable<'_> for bool {
    fn read<const PLAIN: bool>(r: &mut Reader<'_>, _: &XRef<'_>) -> Option<Self> {
        match r.skip_plain::<bool>()? {
            b"true" => Some(true),
            b"false" => Some(false),
            _ => None,
        }
    }
}

object!(bool, Boolean);

#[cfg(test)]
mod tests {
    use crate::reader::Reader;

    #[test]
    fn bool_true() {
        assert_eq!(
            Reader::new("true".as_bytes())
                .read_without_xref::<bool>()
                .unwrap(),
            true
        );
    }

    #[test]
    fn bool_false() {
        assert_eq!(
            Reader::new("false".as_bytes())
                .read_without_xref::<bool>()
                .unwrap(),
            false
        );
    }

    #[test]
    fn bool_trailing() {
        assert_eq!(
            Reader::new("trueabdf".as_bytes())
                .read_without_xref::<bool>()
                .unwrap(),
            true
        );
    }
}
