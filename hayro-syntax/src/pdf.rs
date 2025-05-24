use crate::Data;
use crate::document::page::Pages;
use crate::file::trailer::root_trailer;
use crate::file::xref::{XRef, root_xref};
use crate::object::Object;
use crate::object::dict::Dict;
use crate::object::dict::keys::{ENCRYPT, PAGES, ROOT};
use crate::pdf::PdfError::{EncryptionError, OtherError};

pub struct Pdf<'a> {
    xref: XRef<'a>,
    trailer: Dict<'a>,
    catalog: Dict<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PdfError {
    EncryptionError,
    OtherError,
}

impl<'a> Pdf<'a> {
    pub fn new(data: &'a Data<'a>) -> Result<Self, PdfError> {
        let xref = root_xref(data).ok_or(OtherError)?;
        let trailer = root_trailer(data.get(), &xref).ok_or(OtherError)?;

        if trailer.contains_key(ENCRYPT) {
            return Err(EncryptionError);
        }

        let catalog = trailer.get(ROOT).ok_or(OtherError)?;

        Ok(Self {
            xref,
            trailer,
            catalog,
        })
    }

    /// Return the number of objects present in the PDF file.
    pub fn len(&self) -> usize {
        self.xref.len()
    }

    /// Return an iterator over all objects defined in the PDF file.
    pub fn objects(&'_ self) -> impl IntoIterator<Item = Object<'a>> + '_ {
        self.xref.objects()
    }

    /// Return the trailer dictionary of the PDF.
    pub fn trailer(&self) -> &Dict<'a> {
        &self.trailer
    }

    /// Return the catalog dictionary of the PDF.
    pub fn catalog(&self) -> &Dict<'a> {
        &self.catalog
    }

    pub fn pages(&self) -> Result<Pages<'a>, PdfError> {
        self.catalog
            .get::<Dict>(PAGES)
            .and_then(|p| Pages::new(p))
            .ok_or(OtherError)
    }
}
