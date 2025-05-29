use crate::file::xref::XRef;
use crate::object::array::Array;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader};

#[derive(Clone, Copy, Debug)]
pub struct Rect(kurbo::Rect);

impl Rect {
    pub fn get(&self) -> kurbo::Rect {
        self.0
    }
}

impl Readable<'_> for Rect {
    fn read<const PLAIN: bool>(r: &mut Reader<'_>, _: &XRef) -> Option<Self> {
        let arr = r.read_without_xref::<Array>()?;
        from_arr(&arr)
    }
}

fn from_arr(array: &Array) -> Option<Rect> {
    let mut iter = array.iter::<f32>();

    Some(Rect(kurbo::Rect::new(
        iter.next()? as f64,
        iter.next()? as f64,
        iter.next()? as f64,
        iter.next()? as f64,
    )))
}

impl TryFrom<Object<'_>> for Rect {
    type Error = ();

    fn try_from(value: Object<'_>) -> Result<Self, Self::Error> {
        match value {
            Object::Array(arr) => from_arr(&arr).ok_or(()),
            _ => Err(()),
        }
    }
}

impl ObjectLike<'_> for Rect {}
