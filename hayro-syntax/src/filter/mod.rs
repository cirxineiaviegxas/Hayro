mod ascii_85;
mod ascii_hex;
mod dct;
mod lzw_flate;
mod run_length;

use crate::Result;
use crate::file::xref::XRef;
use crate::object::dict::Dict;
use crate::object::name::Name;
use crate::object::name::names::*;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader};
use snafu::{OptionExt, whatever};

pub fn apply_filter(data: &[u8], filter: Filter, params: Option<&Dict>) -> Result<Vec<u8>> {
    filter.apply(data, params)
}

#[derive(Debug, Copy, Clone)]
pub enum Filter {
    AsciiHexDecode,
    Ascii85Decode,
    LzwDecode,
    FlateDecode,
    RunLengthDecode,
    CcittFaxDecode,
    Jbig2Decode,
    DctDecode,
    JpxDecode,
    Crypt,
}

impl Filter {
    fn debug_name(&self) -> &'static str {
        match self {
            Filter::AsciiHexDecode => "ascii_hex",
            Filter::Ascii85Decode => "ascii_85",
            Filter::LzwDecode => "lzw",
            Filter::FlateDecode => "flate",
            Filter::RunLengthDecode => "run-length",
            Filter::CcittFaxDecode => "ccit_fax",
            Filter::Jbig2Decode => "jbig2",
            Filter::DctDecode => "dct",
            Filter::JpxDecode => "jpx",
            Filter::Crypt => "crypt",
        }
    }

    pub fn apply(&self, data: &[u8], params: Option<&Dict>) -> Result<Vec<u8>> {
        let applied = match self {
            Filter::AsciiHexDecode => ascii_hex::decode(data),
            Filter::Ascii85Decode => ascii_85::decode(data),
            Filter::RunLengthDecode => run_length::decode(data),
            Filter::LzwDecode => lzw_flate::lzw::decode(data, params),
            Filter::DctDecode => dct::decode(data, params),
            Filter::FlateDecode => lzw_flate::flate::decode(data, params),
            _ => {
                whatever!("the {} filter is not supported", self.debug_name());
            }
        };

        applied
            .with_whatever_context(|| format!("failed to apply the {} filter", self.debug_name()))
    }
}

impl<'a> Readable<'a> for Filter {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        r.read::<PLAIN, Name>(xref).and_then(|n| n.try_into().ok())
    }
}

impl ObjectLike<'_> for Filter {
    const STATIC_NAME: &'static str = "Filter";
}

impl TryFrom<Object<'_>> for Filter {
    type Error = ();

    fn try_from(value: Object<'_>) -> std::result::Result<Self, Self::Error> {
        match value {
            Object::Name(n) => n.try_into(),
            _ => Err(()),
        }
    }
}

impl TryFrom<Name<'_>> for Filter {
    type Error = ();

    fn try_from(value: Name) -> std::result::Result<Self, Self::Error> {
        match value.as_ref() {
            ASCII_HEX_DECODE => Ok(Filter::AsciiHexDecode),
            ASCII85_DECODE => Ok(Filter::Ascii85Decode),
            LZW_DECODE => Ok(Filter::LzwDecode),
            FLATE_DECODE => Ok(Filter::FlateDecode),
            RUN_LENGTH_DECODE => Ok(Filter::RunLengthDecode),
            CCITTFAX_DECODE => Ok(Filter::CcittFaxDecode),
            JBIG2_DECODE => Ok(Filter::Jbig2Decode),
            DCT_DECODE => Ok(Filter::DctDecode),
            JPX_DECODE => Ok(Filter::JpxDecode),
            CRYPT => Ok(Filter::Crypt),
            _ => Err(()),
        }
    }
}
