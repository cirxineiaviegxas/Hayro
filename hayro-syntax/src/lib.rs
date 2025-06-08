/*!
A low-level library for reading PDF files.

This crate implements most of the points in the `Syntax` chapter of the PDF reference, and therefore
serves as a very good basis for building various abstractions on top of it, without having to reimplement
the PDF parsing logic. 

This crate does not provide more high-level functionality, such as parsing fonts or color spaces.
Such functionality is out-of-scope for `hayro-syntax`, since this crate is supposed to be
as light-weight and application-agnostic as possible. Functionality-wise, this crate is therefore
pretty much feature-complete, though more low-level APIs might be added in the future.

# Example
This short example shows how you can iterate over the operations of the content stream of all pages
in a PDF file.
```rust
use std::path::PathBuf;
use std::sync::Arc;
use hayro_syntax::pdf::Pdf;

let data = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hayro-render/pdfs/text_with_rise.pdf")).unwrap();
let pdf = Pdf::new(Arc::new(data)).unwrap();
let pages = pdf.pages().unwrap();

for page in pages {
    for op in page.typed_operations() {
        println!("{:?}", op);
    }
}
```

# Features
The supported features include:
- Parsing the xref table in all of its possible formats, including xref streams.
- Parsing of all objects (also in object streams).
- Parsing and evaluating of PDF functions.
- Parsing and decoding PDF streams.
- Iterating over pages in a PDF as well as their content streams in a typed fashion.
- The crate is very lightweight in comparison to other PDF crates, at least if you don't enable
  the jpeg2000 feature.

# Limitations
I would like to highlight the following limitations:

- There are still features missing, for example, support for encrypted PDFs. In addition to that,
  many properties (like page annotations) are currently not exposed.
- This crate is for read-only processing, you cannot directly use it to manipulate PDF files. 
  If you need to do that, there are other crates in the Rust ecosystem that are suitable for this.
- I do want to note that the main reason this crate exists is to serve as a foundation for 
  `hayro-render`. Therefore, I am not planning on adding many other features that aren't needed
  to rasterize PDFs. But I am open to feedback, and if the crate covers everything
  you need, you are more than free to use it directly!

# Cargo features
This crate has one feature, `jpeg2000`. PDF allows for the insertion of JPEG2000 images. However,
unfortunately, JPEG2000 is a very complicated format. There exists a Rust crate that allows decoding
such images (which is also used by `hayro-syntax`), but it is a very heavy dependency, has a lot of
unsafe code (due to having been ported with `c2rust`), and also has a dependency on libc, meaning that you
might be restricted in the targets you can build to. Because of this, I recommend not enabling this
feature, unless you absolutely need to be able to support such images.
*/

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use self::object::ObjectIdentifier;
use crate::object::stream::Stream;
use crate::xref::XRef;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

pub mod bit_reader;
pub mod content;
pub mod document;
pub mod filter;
pub mod function;
pub mod object;
pub mod pdf;
pub mod reader;
pub mod trivia;
pub(crate) mod util;
pub mod xref;

const NUM_SLOTS: usize = 10000;

/// A container for the bytes of a PDF file.
pub type PdfData = Arc<dyn AsRef<[u8]> + Send + Sync>;

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
pub struct Data {
    data: PdfData,
    slots: Vec<OnceCell<Option<Vec<u8>>>>,
    map: Mutex<HashMap<ObjectIdentifier, usize>>,
    counter: AtomicUsize,
}

impl Debug for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data {{ ... }}")
    }
}

impl Data {
    /// Create a new `Data` structure.
    pub fn new(data: PdfData) -> Self {
        let map = Mutex::new(HashMap::new());
        let slots = vec![OnceCell::new(); NUM_SLOTS];
        let counter = AtomicUsize::new(0);

        Self {
            data,
            slots,
            map,
            counter,
        }
    }

    /// Get access to the original data of the PDF.
    pub(crate) fn get(&self) -> &[u8] {
        self.data.as_ref().as_ref()
    }

    /// Get access to the data of a decoded object stream.
    pub(crate) fn get_with(&self, id: ObjectIdentifier, xref: &XRef) -> Option<&[u8]> {
        if let Some(idx) = self.map.lock().unwrap().get(&id) {
            self.slots[*idx].get()?.as_deref()
        } else {
            let mut idx = self.counter.load(std::sync::atomic::Ordering::SeqCst);

            if idx >= NUM_SLOTS {
                None
            } else {
                self.map.lock().unwrap().insert(id, idx);

                let stream = xref.get::<Stream>(id)?;
                self.slots[idx].set(stream.decoded()).unwrap();

                let val = self.slots[idx].get().unwrap().as_deref();
                idx += 1;

                self.counter.store(idx, std::sync::atomic::Ordering::SeqCst);

                val
            }
        }
    }
}
