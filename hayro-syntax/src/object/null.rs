use crate::file::xref::XRef;
use crate::object;
use crate::object::Object;
use crate::reader::{Readable, Reader, Skippable};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Null;

object!(Null, Null);

impl Skippable for Null {
    fn skip<const PLAIN: bool>(r: &mut Reader) -> Option<()> {
        r.forward_tag(b"null")
    }
}

impl Readable<'_> for Null {
    fn read<const PLAIN: bool>(r: &mut Reader, _: &XRef<'_>) -> Option<Self> {
        Self::skip::<true>(r)?;

        Some(Null)
    }
}

#[cfg(test)]
mod tests {
    use crate::object::null::Null;
    use crate::reader::Reader;

    #[test]
    fn null() {
        assert_eq!(
            Reader::new("null".as_bytes()).read_without_xref::<Null>().unwrap(),
            Null
        );
    }

    #[test]
    fn null_trailing() {
        assert_eq!(
            Reader::new("nullabs".as_bytes())
                .read_without_xref::<Null>()
                .unwrap(),
            Null
        );
    }
}
