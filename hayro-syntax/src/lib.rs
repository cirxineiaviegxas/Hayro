#![forbid(unsafe_code)]

use self::object::ObjectIdentifier;
use crate::file::xref::XRef;
use crate::object::stream::Stream;
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

pub mod bit;
pub mod content;
pub mod document;
pub mod file;
pub mod filter;
pub mod function;
pub mod object;
pub mod pdf;
pub mod reader;
pub mod trivia;
pub(crate) mod util;

const NUM_SLOTS: usize = 10000;

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
    slots: Vec<OnceCell<Option<Vec<u8>>>>,
    map: RefCell<HashMap<ObjectIdentifier, usize>>,
    counter: RefCell<usize>,
}

impl Debug for Data<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data {{ ... }}")
    }
}

impl<'a> Data<'a> {
    /// Create a new `Data` structure.
    pub fn new(data: &'a [u8]) -> Self {
        let map = RefCell::new(HashMap::new());
        let slots = vec![OnceCell::new(); NUM_SLOTS];
        let counter = RefCell::new(0);

        Self {
            data,
            slots,
            map,
            counter,
        }
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
        if let Some(idx) = self.map.borrow().get(&id) {
            self.slots[*idx].get()?.as_deref()
        } else {
            let mut idx = self.counter.borrow_mut();

            if *idx >= NUM_SLOTS {
                None
            } else {
                self.map.borrow_mut().insert(id, *idx);

                let stream = xref.get::<Stream>(id)?;
                self.slots[*idx].set(stream.decoded()).unwrap();

                let val = self.slots[*idx].get().unwrap().as_deref();
                *idx += 1;

                val
            }
        }
    }
}
