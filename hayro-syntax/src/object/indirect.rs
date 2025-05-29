use crate::file::xref::XRef;
use crate::object::{ObjectIdentifier, ObjectLike};
use crate::reader::{Readable, Reader, Skippable};

pub(crate) struct IndirectObject<T> {
    id: ObjectIdentifier,
    inner: T,
}

impl<T> IndirectObject<T> {
    pub(crate) fn get(self) -> T {
        self.inner
    }

    pub(crate) fn id(&self) -> &ObjectIdentifier {
        &self.id
    }
}

impl<'a, T> Readable<'a> for IndirectObject<T>
where
    T: ObjectLike<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        let id = r.read_without_xref::<ObjectIdentifier>()?;
        r.skip_white_spaces_and_comments();
        let inner = r.read_with_xref::<T>(xref)?;
        r.skip_white_spaces_and_comments();
        // We are lenient and don't require it.
        r.forward_tag(b"endobj");

        Some(Self { id, inner })
    }
}

impl<T> Skippable for IndirectObject<T>
where
    T: Skippable,
{
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.skip_plain::<ObjectIdentifier>()?;
        r.skip_white_spaces_and_comments();
        r.skip_non_plain::<T>()?;
        r.skip_white_spaces_and_comments();
        // We are lenient and don't require it.
        r.forward_tag(b"endobj");

        Some(())
    }
}
