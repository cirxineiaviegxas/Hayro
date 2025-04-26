use crate::Data;
use crate::filter::Filter;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::name::Name;
use crate::object::null::Null;
use crate::object::number::Number;
use crate::object::stream::Stream;
use crate::object::string::{HexString, LiteralString, String};
use crate::object::{Object, ObjectIdentifier, ObjectLike};
use log::warn;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub(crate) trait Static: Sized {
    type STATIC: Sized + Clone + 'static;
}

macro_rules! static_impl {
    ($t:ident) => {
        impl Static for $t {
            type STATIC = $t;
        }
    };
}

macro_rules! static_impl_l {
    ($t:ident) => {
        impl Static for $t<'_> {
            type STATIC = $t<'static>;
        }
    };
}

static_impl!(Null);
static_impl!(f32);
static_impl!(i32);
static_impl!(u32);
static_impl!(usize);
static_impl!(u8);
static_impl!(bool);
static_impl!(Number);
static_impl!(Filter);

static_impl_l!(Array);
static_impl_l!(Object);
static_impl_l!(Dict);
static_impl_l!(Name);
static_impl_l!(String);
static_impl_l!(HexString);
static_impl_l!(LiteralString);
static_impl_l!(Stream);

pub struct Cache<'a> {
    entries: RwLock<HashMap<ObjectIdentifier, Arc<dyn Any>>>,
    _data: &'a Data<'a>,
}

impl<'a> Cache<'a> {
    pub(crate) fn new(data: &'a Data<'a>) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            _data: data,
        }
    }

    pub(crate) fn contains_key(&self, id: ObjectIdentifier) -> bool {
        self.entries.read().unwrap().contains_key(&id)
    }

    pub(crate) fn insert<T>(&self, id: ObjectIdentifier, entry: T)
    where
        T: ObjectLike<'a>,
    {
        let ptr = &entry as *const T as *const T::STATIC;
        let converted: T::STATIC = unsafe { ptr.read() };

        let mut entries = self.entries.write().unwrap();
        entries.insert(id, Arc::new(converted));
    }

    pub(crate) fn get<T>(&self, id: ObjectIdentifier) -> Option<T>
    where
        T: ObjectLike<'a>,
    {
        if let Some(entry) = self.entries.write().unwrap().get(&id).cloned() {
            if let Some(val) = entry.downcast_ref::<T::STATIC>().cloned() {
                let ptr = &val as *const T::STATIC as *const T;
                let converted: T = unsafe { ptr.read() };
                return Some(converted);
            } else {
                warn!(
                    "attempted to read cache entry {}, but the types didn't match.",
                    T::STATIC_NAME
                );
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::Data;
    use crate::cache::Cache;
    use crate::object::{ObjectIdentifier, string};
    use crate::reader::Reader;

    // TODO: Add proper tests

    #[test]
    fn cache() {
        let input = b"(Hi this is a string)";
        let data = Data::new(*&input);
        let cache = Cache::new(&data);

        let mut r = Reader::new(input);
        let id = ObjectIdentifier::new(1, 0);
        let str = r.read_without_xref::<string::String>().unwrap();

        cache.insert(id, str);

        let res = cache.get::<string::String>(id);
        assert!(res.is_some());
    }
}
