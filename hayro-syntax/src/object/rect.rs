use crate::file::xref::XRef;
use crate::object::array::Array;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader};
use log::warn;
use smallvec::SmallVec;

#[derive(Clone, Copy, Debug)]
pub struct Rect(kurbo::Rect);

impl Rect {
    pub fn get(&self) -> kurbo::Rect {
        self.0
    }
}

impl Readable<'_> for Rect {
    fn read<const PLAIN: bool>(r: &mut Reader<'_>, _: &XRef<'_>) -> Option<Self> {
        let arr = r.read_without_xref::<Array>()?;
        from_arr(&arr)
    }
}

fn from_arr(array: &Array) -> Option<Rect> {
    let c: SmallVec<[f32; 4]> = array.iter::<f32>().collect();

    if c.len() != 4 {
        warn!("encountered rect with no 4 values");

        return None;
    }

    Some(Rect(kurbo::Rect::new(
        c[0] as f64,
        c[1] as f64,
        c[2] as f64,
        c[3] as f64,
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
