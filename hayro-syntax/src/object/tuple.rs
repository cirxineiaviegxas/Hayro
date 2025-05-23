use crate::file::xref::XRef;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader};
use std::fmt::Debug;

// Note that tuples don't correspond to any specific PDF object. Instead, they simply
// represent a number of PDF objects that are only separated by whitespaces, i.e.
// in an array. We only have those implementations so that it is easier to iterate
// over tuples of items in a PDF array, which is something that happens quite often.

impl<'a, T, U> Readable<'a> for (T, U)
where
    T: Readable<'a>,
    U: Readable<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = T::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let u = U::read::<PLAIN>(r, xref)?;
        Some((t, u))
    }
}

impl<'a, T, U> TryFrom<Object<'a>> for (T, U)
where
    T: Readable<'a>,
    U: Readable<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U> ObjectLike<'a> for (T, U)
where
    T: Readable<'a> + Debug + Clone,
    U: Readable<'a> + Debug + Clone,
{
}

impl<'a, T, U, V> Readable<'a> for (T, U, V)
where
    T: Readable<'a>,
    U: Readable<'a>,
    V: Readable<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = T::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let u = U::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let v = V::read::<PLAIN>(r, xref)?;

        Some((t, u, v))
    }
}

impl<'a, T, U, V> TryFrom<Object<'a>> for (T, U, V)
where
    T: Readable<'a>,
    U: Readable<'a>,
    V: Readable<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U, V> ObjectLike<'a> for (T, U, V)
where
    T: Readable<'a> + Debug + Clone,
    U: Readable<'a> + Debug + Clone,
    V: Readable<'a> + Debug + Clone,
{
}

impl<'a, T, U, V, W> Readable<'a> for (T, U, V, W)
where
    T: Readable<'a>,
    U: Readable<'a>,
    V: Readable<'a>,
    W: Readable<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = T::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let u = U::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let v = V::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let w = W::read::<PLAIN>(r, xref)?;

        Some((t, u, v, w))
    }
}

impl<'a, T, U, V, W> TryFrom<Object<'a>> for (T, U, V, W)
where
    T: Readable<'a>,
    U: Readable<'a>,
    V: Readable<'a>,
    W: Readable<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U, V, W> ObjectLike<'a> for (T, U, V, W)
where
    T: Readable<'a> + Debug + Clone,
    U: Readable<'a> + Debug + Clone,
    V: Readable<'a> + Debug + Clone,
    W: Readable<'a> + Debug + Clone,
{
}

impl<'a, T, U, V, W, X> Readable<'a> for (T, U, V, W, X)
where
    T: Readable<'a>,
    U: Readable<'a>,
    V: Readable<'a>,
    W: Readable<'a>,
    X: Readable<'a>,
{
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        r.skip_white_spaces_and_comments();
        let t = T::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let u = U::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let v = V::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let w = W::read::<PLAIN>(r, xref)?;
        r.skip_white_spaces_and_comments();
        let x = X::read::<PLAIN>(r, xref)?;

        Some((t, u, v, w, x))
    }
}

impl<'a, T, U, V, W, X> TryFrom<Object<'a>> for (T, U, V, W, X)
where
    T: Readable<'a>,
    U: Readable<'a>,
    V: Readable<'a>,
    W: Readable<'a>,
    X: Readable<'a>,
{
    type Error = ();

    fn try_from(_: Object<'a>) -> Result<Self, Self::Error> {
        Err(())
    }
}

impl<'a, T, U, V, W, X> ObjectLike<'a> for (T, U, V, W, X)
where
    T: Readable<'a> + Debug + Clone,
    U: Readable<'a> + Debug + Clone,
    V: Readable<'a> + Debug + Clone,
    W: Readable<'a> + Debug + Clone,
    X: Readable<'a> + Debug + Clone,
{
}
