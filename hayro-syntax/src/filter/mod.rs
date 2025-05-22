mod ascii_85;
mod ascii_hex;
mod ccitt;
mod dct;
mod jpx;
mod lzw_flate;
mod run_length;

use crate::OptionLog;
use crate::file::xref::XRef;
use crate::object::dict::Dict;
use crate::object::dict::keys::*;
use crate::object::name::Name;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader};
use log::warn;

pub fn apply_filter(data: &[u8], filter: Filter, params: Option<&Dict>) -> Option<FilterResult> {
    filter.apply(data, params.cloned().unwrap_or_default())
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

pub enum ImageColorSpace {
    Gray,
    Rgb,
    Cmyk,
}

pub struct FilterResult {
    pub data: Vec<u8>,
    pub color_space: Option<ImageColorSpace>,
    pub bits_per_component: Option<u8>,
}

impl FilterResult {
    pub fn from_data(data: Vec<u8>) -> Self {
        Self {
            data,
            color_space: None,
            bits_per_component: None,
        }
    }
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

    pub fn apply(&self, data: &[u8], params: Dict) -> Option<FilterResult> {
        match self {
            Filter::AsciiHexDecode => ascii_hex::decode(data).map(FilterResult::from_data),
            Filter::Ascii85Decode => ascii_85::decode(data).map(FilterResult::from_data),
            Filter::RunLengthDecode => run_length::decode(data).map(FilterResult::from_data),
            Filter::LzwDecode => lzw_flate::lzw::decode(data, params).map(FilterResult::from_data),
            Filter::DctDecode => dct::decode(data, params).map(FilterResult::from_data),
            Filter::FlateDecode => {
                lzw_flate::flate::decode(data, params).map(FilterResult::from_data)
            }
            Filter::CcittFaxDecode => ccitt::decode(data, params).map(FilterResult::from_data),
            Filter::JpxDecode => jpx::decode(data),
            _ => None,
        }
        .warn_none(&format!("failed to apply filter {}", self.debug_name()))
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
        match value {
            ASCII_HEX_DECODE | ASCII_HEX_DECODE_ABBREVIATION => Ok(Filter::AsciiHexDecode),
            ASCII85_DECODE | ASCII85_DECODE_ABBREVIATION => Ok(Filter::Ascii85Decode),
            LZW_DECODE | LZW_DECODE_ABBREVIATION => Ok(Filter::LzwDecode),
            FLATE_DECODE | FLATE_DECODE_ABBREVIATION => Ok(Filter::FlateDecode),
            RUN_LENGTH_DECODE | RUN_LENGTH_DECODE_ABBREVIATION => Ok(Filter::RunLengthDecode),
            CCITTFAX_DECODE | CCITTFAX_DECODE_ABBREVIATION => Ok(Filter::CcittFaxDecode),
            JBIG2_DECODE => Ok(Filter::Jbig2Decode),
            DCT_DECODE | DCT_DECODE_ABBREVIATION => Ok(Filter::DctDecode),
            JPX_DECODE => Ok(Filter::JpxDecode),
            CRYPT => Ok(Filter::Crypt),
            _ => {
                warn!("unknown filter: {}", value.as_str());

                Err(())
            }
        }
    }
}
