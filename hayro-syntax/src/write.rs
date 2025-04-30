use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::name::Name;
use crate::object::null::Null;
use crate::object::number::{InternalNumber, Number};
use crate::object::r#ref::MaybeRef;
use crate::object::stream::Stream;
use crate::object::{Object, string};
use pdf_writer::{Chunk, Obj, Ref};

pub fn display<T>(item: &T) -> Option<String>
where
    T: WriteIndirect,
{
    let mut chunk = Chunk::new();
    item.write_indirect(&mut chunk, Ref::new(1));
    let start = 0;
    let end = chunk.len() - 6;

    std::str::from_utf8(&chunk.as_bytes()[start..end])
        .ok()
        .map(|s| s.to_string())
}

pub trait WriteIndirect {
    fn write_indirect(&self, chunk: &mut Chunk, id: Ref);
}

impl<T> WriteIndirect for T
where
    T: WriteDirect,
{
    fn write_indirect(&self, chunk: &mut Chunk, id: Ref) {
        self.write_direct(chunk.indirect(id))
    }
}

impl WriteIndirect for Stream<'_> {
    fn write_indirect(&self, chunk: &mut Chunk, id: Ref) {
        // TODO: Add filters
        chunk.stream(id, self.raw_data());
    }
}

pub trait WriteDirect {
    fn write_direct(&self, obj: Obj);
}

impl WriteDirect for Object<'_> {
    fn write_direct(&self, obj: Obj) {
        match self {
            Object::Null(n) => n.write_direct(obj),
            Object::Boolean(b) => b.write_direct(obj),
            Object::Number(n) => n.write_direct(obj),
            Object::String(s) => s.write_direct(obj),
            Object::Name(n) => n.write_direct(obj),
            Object::Dict(d) => d.write_direct(obj),
            Object::Array(a) => a.write_direct(obj),
            Object::Stream(_) => unimplemented!(),
        }
    }
}

impl WriteDirect for string::String<'_> {
    fn write_direct(&self, obj: Obj) {
        obj.primitive(pdf_writer::Str(&self.get()))
    }
}

impl<T> WriteDirect for MaybeRef<T>
where
    T: WriteDirect,
{
    fn write_direct(&self, obj: Obj) {
        match self {
            // TODO: This will not preserve the generation number!
            MaybeRef::Ref(r) => obj.primitive(Ref::new(r.obj_number)),
            MaybeRef::NotRef(t) => t.write_direct(obj),
        }
    }
}

impl WriteDirect for Number {
    fn write_direct(&self, obj: Obj) {
        match self.0 {
            InternalNumber::Real(r) => obj.primitive(r),
            InternalNumber::Integer(i) => obj.primitive(i),
        }
    }
}

impl WriteDirect for Null {
    fn write_direct(&self, obj: Obj) {
        obj.primitive(pdf_writer::Null)
    }
}

impl WriteDirect for Name<'_> {
    fn write_direct(&self, obj: Obj) {
        obj.primitive(pdf_writer::Name(&self.as_ref()))
    }
}

impl WriteDirect for bool {
    fn write_direct(&self, obj: Obj) {
        obj.primitive(self)
    }
}

impl WriteDirect for Dict<'_> {
    fn write_direct(&self, obj: Obj) {
        let mut dict = obj.dict();

        for key in self.keys() {
            let obj = dict.insert(pdf_writer::Name(key.as_ref()));
            let entry = self.get_raw::<Object>(key).unwrap();
            entry.write_direct(obj);
        }
    }
}

impl WriteDirect for Array<'_> {
    fn write_direct(&self, obj: Obj) {
        let mut arr = obj.array();

        self.raw_iter().for_each(|i| {
            let obj = arr.push();
            i.write_direct(obj);
        });
    }
}
