//! The starting point for reading PDF files.

use crate::PdfData;
use crate::document::page::{Page, Pages};
use crate::object::Object;
use crate::xref::{XRef, fallback, root_xref};

/// A PDF file.
pub struct Pdf {
    xref: XRef,
}

impl Pdf {
    /// Try to read the given PDF file.
    ///
    /// Returns `None` if it was unable to read it.
    pub fn new(data: PdfData) -> Option<Self> {
        let xref = root_xref(data.clone()).or_else(|| fallback(data))?;

        Some(Self { xref })
    }

    /// Return the number of objects present in the PDF file.
    pub fn len(&self) -> usize {
        self.xref.len()
    }

    /// Return an iterator over all objects defined in the PDF file.
    pub fn objects(&self) -> impl IntoIterator<Item = Object> {
        self.xref.objects()
    }

    /// Return the pages of the PDF file.
    pub fn pages(&self) -> Option<Vec<Page>> {
        self.xref
            .get(self.xref.trailer_data().pages_ref)
            .and_then(|p| Pages::new(p, &self.xref).map(|p| p.pages))
    }
}
