use crate::PdfData;
use crate::document::page::Pages;
use crate::object::Object;
use crate::pdf::PdfError::OtherError;
use crate::xref::{XRef, fallback, root_xref};

pub struct Pdf {
    xref: XRef,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PdfError {
    EncryptionError,
    OtherError,
}

impl Pdf {
    pub fn new(data: PdfData) -> Result<Self, PdfError> {
        let xref = root_xref(data.clone())
            .or_else(|| fallback(data))
            .ok_or(OtherError)?;

        Ok(Self { xref })
    }

    /// Return the number of objects present in the PDF file.
    pub fn len(&self) -> usize {
        self.xref.len()
    }

    /// Return an iterator over all objects defined in the PDF file.
    pub fn objects(&self) -> impl IntoIterator<Item = Object> {
        self.xref.objects()
    }

    pub fn pages(&self) -> Result<Pages, PdfError> {
        self.xref
            .get(self.xref.trailer_data().pages_ref)
            .and_then(|p| Pages::new(p, &self.xref))
            .ok_or(OtherError)
    }
}
