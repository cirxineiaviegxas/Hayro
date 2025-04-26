use crate::file::xref::XRef;
use crate::object::ObjectIdentifier;
use crate::object::ObjectLike;
use crate::reader::{Readable, Reader, Skippable};
use std::fmt::{Debug, Formatter};

/// A reference to an object.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct ObjRef {
    pub obj_number: i32,
    pub gen_number: i32,
}

impl ObjRef {
    pub fn new(obj_number: i32, gen_number: i32) -> Self {
        Self {
            obj_number,
            gen_number,
        }
    }
}

impl From<ObjRef> for ObjectIdentifier {
    fn from(value: ObjRef) -> Self {
        ObjectIdentifier::new(value.obj_number, value.gen_number)
    }
}

impl Skippable for ObjRef {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.skip_non_plain::<i32>()?;
        r.skip_white_spaces();
        r.skip_non_plain::<i32>()?;
        r.skip_white_spaces();
        r.forward_tag(b"R")?;

        Some(())
    }
}

impl Readable<'_> for ObjRef {
    fn read<const PLAIN: bool>(r: &mut Reader<'_>, _: &XRef<'_>) -> Option<Self> {
        let obj_ref = r.read_without_xref::<i32>()?;
        r.skip_white_spaces();
        let gen_num = r.read_without_xref::<i32>()?;
        r.skip_white_spaces();
        r.forward_tag(b"R")?;

        Some(Self::new(obj_ref, gen_num))
    }
}

pub(crate) enum MaybeRef<T> {
    Ref(ObjRef),
    NotRef(T),
}

impl<'a, T> MaybeRef<T>
where
    T: ObjectLike<'a>,
{
    pub(crate) fn resolve(self, xref: &XRef<'a>) -> Option<T> {
        match self {
            MaybeRef::Ref(r) => xref.get::<T>(r.into()),
            MaybeRef::NotRef(t) => Some(t),
        }
    }
}

impl<T> TryFrom<MaybeRef<T>> for ObjRef {
    type Error = ();

    fn try_from(value: MaybeRef<T>) -> Result<Self, Self::Error> {
        match value {
            MaybeRef::Ref(r) => Ok(r),
            MaybeRef::NotRef(_) => Err(()),
        }
    }
}

impl<T> Debug for MaybeRef<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MaybeRef::Ref(r) => write!(f, "{:?}", r),
            MaybeRef::NotRef(nr) => write!(f, "{:?}", nr),
        }
    }
}

impl<T> Skippable for MaybeRef<T>
where
    T: Skippable,
{
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.skip::<PLAIN, ObjRef>()
            .or_else(|| r.skip::<PLAIN, T>())
            .map(|_| {})
    }
}

impl<'a, T> Readable<'a> for MaybeRef<T>
where
    T: Readable<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        if let Some(obj) = r.read::<PLAIN, ObjRef>(xref) {
            Some(Self::Ref(obj))
        } else {
            Some(Self::NotRef(r.read::<PLAIN, T>(xref)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::object::r#ref::ObjRef;
    use crate::reader::Reader;

    #[test]
    fn ref_1() {
        assert_eq!(
            Reader::new("34 1 R".as_bytes())
                .read_without_xref::<ObjRef>()
                .unwrap(),
            ObjRef::new(34, 1)
        );
    }

    #[test]
    fn ref_trailing() {
        assert_eq!(
            Reader::new("256 0 R (hi)".as_bytes())
                .read_without_xref::<ObjRef>()
                .unwrap(),
            ObjRef::new(256, 0)
        );
    }

    #[test]
    fn ref_invalid_1() {
        assert!(
            Reader::new("256 R".as_bytes())
                .read_without_xref::<ObjRef>()
                .is_none()
        );
    }

    #[test]
    fn ref_invalid_2() {
        assert!(
            Reader::new("256 257".as_bytes())
                .read_without_xref::<ObjRef>()
                .is_none()
        );
    }
}
