use crate::object::r#ref::MaybeRef;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader};
use crate::xref::XRef;
use std::fmt::Debug;
// Note that tuples don't correspond to any specific PDF object. Instead, they simply
// represent a number of PDF objects that are only separated by whitespaces, i.e.
// in an array. We only have those implementations so that it is easier to iterate
// over tuples of items in a PDF array, which is something that happens quite often.

impl<'a, T, U> Readable<'a> for (T, U)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &'a XRef) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = r.read::<PLAIN, MaybeRef<T>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let u = r.read::<PLAIN, MaybeRef<U>>(xref)?.resolve(xref)?;
        Some((t, u))
    }
}

impl<'a, T, U> TryFrom<Object<'a>> for (T, U)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U> ObjectLike<'a> for (T, U)
where
    T: ObjectLike<'a> + Debug + Clone,
    U: ObjectLike<'a> + Debug + Clone,
{
}

impl<'a, T, U, V> Readable<'a> for (T, U, V)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
    V: ObjectLike<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &'a XRef) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = r.read::<PLAIN, MaybeRef<T>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let u = r.read::<PLAIN, MaybeRef<U>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let v = r.read::<PLAIN, MaybeRef<V>>(xref)?.resolve(xref)?;

        Some((t, u, v))
    }
}

impl<'a, T, U, V> TryFrom<Object<'a>> for (T, U, V)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
    V: ObjectLike<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U, V> ObjectLike<'a> for (T, U, V)
where
    T: ObjectLike<'a> + Debug + Clone,
    U: ObjectLike<'a> + Debug + Clone,
    V: ObjectLike<'a> + Debug + Clone,
{
}

impl<'a, T, U, V, W> Readable<'a> for (T, U, V, W)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
    V: ObjectLike<'a>,
    W: ObjectLike<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &'a XRef) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = r.read::<PLAIN, MaybeRef<T>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let u = r.read::<PLAIN, MaybeRef<U>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let v = r.read::<PLAIN, MaybeRef<V>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let w = r.read::<PLAIN, MaybeRef<W>>(xref)?.resolve(xref)?;

        Some((t, u, v, w))
    }
}

impl<'a, T, U, V, W> TryFrom<Object<'a>> for (T, U, V, W)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
    V: ObjectLike<'a>,
    W: ObjectLike<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U, V, W> ObjectLike<'a> for (T, U, V, W)
where
    T: ObjectLike<'a> + Debug + Clone,
    U: ObjectLike<'a> + Debug + Clone,
    V: ObjectLike<'a> + Debug + Clone,
    W: ObjectLike<'a> + Debug + Clone,
{
}

impl<'a, T, U, V, W, X> Readable<'a> for (T, U, V, W, X)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
    V: ObjectLike<'a>,
    W: ObjectLike<'a>,
    X: ObjectLike<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &'a XRef) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = r.read::<PLAIN, MaybeRef<T>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let u = r.read::<PLAIN, MaybeRef<U>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let v = r.read::<PLAIN, MaybeRef<V>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let w = r.read::<PLAIN, MaybeRef<W>>(xref)?.resolve(xref)?;
        r.skip_white_spaces_and_comments();
        let x = r.read::<PLAIN, MaybeRef<X>>(xref)?.resolve(xref)?;

        Some((t, u, v, w, x))
    }
}

impl<'a, T, U, V, W, X> TryFrom<Object<'a>> for (T, U, V, W, X)
where
    T: ObjectLike<'a>,
    U: ObjectLike<'a>,
    V: ObjectLike<'a>,
    W: ObjectLike<'a>,
    X: ObjectLike<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U, V, W, X> ObjectLike<'a> for (T, U, V, W, X)
where
    T: ObjectLike<'a> + Debug + Clone,
    U: ObjectLike<'a> + Debug + Clone,
    V: ObjectLike<'a> + Debug + Clone,
    W: ObjectLike<'a> + Debug + Clone,
    X: ObjectLike<'a> + Debug + Clone,
{
}
