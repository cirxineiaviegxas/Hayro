use self::object::ObjectIdentifier;
use crate::file::xref::XRef;
use crate::object::Object;
use crate::object::stream::Stream;
use crate::reader::Reader;
use log::{error, trace, warn};
use snafu::Whatever;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

mod cache;
pub mod content;
mod document;
pub mod file;
pub mod filter;
pub mod object;
pub mod pdf;
pub(crate) mod reader;
pub mod trivia;
pub mod write;

pub type Result<T> = std::result::Result<T, Whatever>;

/// A structure for storing the data of the PDF.
// To explain further: This crate uses a zero-parse approach, meaning that objects like
// dictionaries or arrays always store the underlying data and parse objects lazily as needed,
// instead of allocating the data and storing it in an owned way. However, the problem is that
// not all data is readily available: Objects can also be stored in an object stream, in which
// case we first need to decode the stream before we can access the data.
//
// The purpose of `Data` is to allow us to access original data as well as maybe decoded data
// by faking the same lifetime, so that we don't run into lifetime issues when dealing with
// PDF objects that actually stem from different data sources.
pub struct Data<'a> {
    data: &'a [u8],
    map: RwLock<HashMap<ObjectIdentifier, Option<Cow<'a, [u8]>>>>,
}

impl<'a> Data<'a> {
    /// Create a new `Data` structure.
    pub fn new(data: &'a [u8]) -> Self {
        let map = RwLock::new(HashMap::new());

        Self { data, map }
    }

    /// Get access to the original data of the PDF.
    pub(crate) fn get(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Get access to the data of a decoded object stream.
    pub(crate) fn get_with<'b>(
        &'b self,
        id: ObjectIdentifier,
        xref: &XRef<'a>,
    ) -> Option<&'b [u8]> {
        let read_lock = self.map.read().unwrap();

        if let Some(b) = read_lock.get(&id) {
            // SAFETY:
            // We need `unsafe` for the below, because we are extending the lifetime of the
            // data that is originally tied to the duration of the read lock, to the lifetime
            // of the `self` reference. However, this is fine for the following reasons:
            // - As mentioned, the data is assigned the lifetime of `self`, and thus
            // no reference we give out can outlive the `Data` struct itself or its 'a
            // lifetime.
            // - Once we add data to `map`, we _never_ update or remove it in any way.
            // We do insert new entries in the hashmap, but the underlying data itself
            // stays at a stable memory location.
            // - We do not hand out mutable references to the underlying data.
            b.as_ref()
                .map(|d| unsafe { std::slice::from_raw_parts(d.as_ptr(), d.len()) })
        } else {
            drop(read_lock);

            let mut write_lock = self.map.write().unwrap();

            write_lock
                .entry(id)
                .or_insert_with(|| {
                    let stream = xref.get::<Stream>(id)?;
                    stream.decoded().ok()
                })
                .as_ref()
                .map(|b| {
                    // SAFETY: See above.
                    unsafe { std::slice::from_raw_parts(b.as_ptr(), b.len()) }
                })
        }
    }
}

pub(crate) trait OptionLog {
    fn warn_none(self, f: &str) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn warn_none(self, f: &str) -> Self {
        self.or_else(|| {
            warn!("{}", f);

            None
        })
    }
}
