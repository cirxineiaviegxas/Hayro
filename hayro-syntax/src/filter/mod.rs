mod ascii_85;
mod ascii_hex;
mod ccit_stream;
mod ccitt;
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
use log::warn;
use snafu::{OptionExt, whatever};

pub fn apply_filter(data: &[u8], filter: Filter, params: Option<&Dict>) -> Result<FilterResult> {
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

pub enum ColorSpace {
    Gray,
    Rgb,
    Cmyk,
}

pub struct FilterResult {
    pub data: Vec<u8>,
    pub color_space: Option<ColorSpace>,
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

    pub fn apply(&self, data: &[u8], params: Dict) -> Result<FilterResult> {
        let applied = match self {
            Filter::AsciiHexDecode => ascii_hex::decode(data).map(FilterResult::from_data),
            Filter::Ascii85Decode => ascii_85::decode(data).map(FilterResult::from_data),
            Filter::RunLengthDecode => run_length::decode(data).map(FilterResult::from_data),
            Filter::LzwDecode => lzw_flate::lzw::decode(data, params).map(FilterResult::from_data),
            Filter::DctDecode => dct::decode(data, params).map(FilterResult::from_data),
            Filter::FlateDecode => {
                lzw_flate::flate::decode(data, params).map(FilterResult::from_data)
            }
            Filter::CcittFaxDecode => {
                ccit_stream::decode(data, params).map(FilterResult::from_data)
            }
            Filter::JpxDecode => {
                // TODO: Make dependency optional to allow compiling to WASM.
                let image = jpeg2k::Image::from_bytes(data).unwrap();
                let components = image.components();
                let cs = match components.len() {
                    1 => Some(ColorSpace::Gray),
                    3 => Some(ColorSpace::Rgb),
                    4 => Some(ColorSpace::Cmyk),
                    _ => None,
                };
                let bpc = components
                    .iter()
                    .fold(std::u32::MIN, |max, c| max.max(c.precision()))
                    as u8;
                let mut components_iters = image
                    .components()
                    .iter()
                    .map(|c| c.data_u8())
                    .collect::<Vec<_>>();
                let mut buf = vec![];

                'outer: loop {
                    for iter in &mut components_iters {
                        if let Some(n) = iter.next() {
                            buf.push(n);
                        } else {
                            break 'outer;
                        }
                    }
                }

                Some(FilterResult {
                    data: buf,
                    color_space: cs,
                    bits_per_component: Some(bpc),
                })
            }
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
            ASCII_HEX_DECODE | b"AHx" => Ok(Filter::AsciiHexDecode),
            ASCII85_DECODE | b"A85" => Ok(Filter::Ascii85Decode),
            LZW_DECODE | b"LZW" => Ok(Filter::LzwDecode),
            FLATE_DECODE | b"Fl" => Ok(Filter::FlateDecode),
            RUN_LENGTH_DECODE | b"RL" => Ok(Filter::RunLengthDecode),
            CCITTFAX_DECODE | b"CCF" => Ok(Filter::CcittFaxDecode),
            JBIG2_DECODE => Ok(Filter::Jbig2Decode),
            DCT_DECODE | b"DCT" => Ok(Filter::DctDecode),
            JPX_DECODE => Ok(Filter::JpxDecode),
            CRYPT => Ok(Filter::Crypt),
            _ => {
                warn!("unknown filter: {}", value.as_str());

                Err(())
            }
        }
    }
}
