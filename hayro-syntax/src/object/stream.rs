use crate::Result;
use crate::file::xref::XRef;
use crate::filter::{Filter, FilterResult, apply_filter};
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{DECODE_PARMS, DP, F, FILTER, LENGTH};
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader, Skippable};
use log::warn;
use std::fmt::{Debug, Formatter};

/// A stream of arbitrary data.
#[derive(Clone, PartialEq)]
pub struct Stream<'a> {
    pub(crate) dict: Dict<'a>,
    data: &'a [u8],
}

impl<'a> Stream<'a> {
    /// Return the raw (potentially with some applied filters) data of the stream.
    pub fn raw_data(&self) -> &'a [u8] {
        self.data
    }

    /// Return the raw (potentially with some applied filters) data of the stream.
    pub fn dict(&self) -> &Dict<'a> {
        &self.dict
    }

    /// Return the decoded data of the stream.
    ///
    /// Note that the result of this method will not be cached, so calling it multiple
    /// times is expensive.
    pub fn decoded(&self) -> Result<Vec<u8>> {
        self.decoded_image().map(|r| r.data)
    }

    pub fn decoded_image(&self) -> Result<FilterResult> {
        if let Some(filter) = self
            .dict
            .get::<Filter>(FILTER)
            .or_else(|| self.dict.get::<Filter>(F))
        {
            let params = self
                .dict
                .get::<Dict>(DECODE_PARMS)
                .or_else(|| self.dict.get::<Dict>(DP));

            Ok(apply_filter(self.data, filter, params.as_ref())?)
        } else if let Some(filters) = self
            .dict
            .get::<Array>(FILTER)
            .or_else(|| self.dict.get::<Array>(F))
        {
            // TODO: Avoid allocation?

            let filters = filters.iter::<Filter>().collect::<Vec<_>>();
            let params = self
                .dict
                .get::<Array>(DECODE_PARMS)
                .or_else(|| self.dict.get::<Array>(DP))
                .map(|a| a.iter::<Object>().collect())
                .unwrap_or(vec![]);

            let mut current: Option<FilterResult> = None;

            for i in 0..filters.len() {
                let params = params.get(i).and_then(|p| p.clone().cast::<Dict>());

                let new = apply_filter(
                    current
                        .as_ref()
                        .map(|c| c.data.as_ref())
                        .unwrap_or(self.data),
                    filters[i],
                    params.as_ref(),
                )?;
                current = Some(new);
            }

            Ok(current.unwrap_or(FilterResult {
                data: self.data.to_vec(),
                color_space: None,
                bits_per_component: None,
            }))
        } else {
            Ok(FilterResult {
                data: self.data.to_vec(),
                color_space: None,
                bits_per_component: None,
            })
        }
    }

    pub fn from_raw(data: &'a [u8], dict: Dict<'a>) -> Self {
        Self { dict, data }
    }
}

impl Debug for Stream<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream (len: {:?})", self.data.len())
    }
}

impl Skippable for Stream<'_> {
    fn skip<const PLAIN: bool>(_: &mut Reader<'_>) -> Option<()> {
        // A stream can never appear in a dict/array, so it should never be skipped.
        unimplemented!()
    }
}

impl<'a> Readable<'a> for Stream<'a> {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        let dict = r.read_with_xref::<Dict>(xref)?;

        if dict.contains_key(F) {
            warn!("encountered stream referencing external file, which is unsupported");

            return None;
        }

        let length = dict.get::<i32>(LENGTH)?;

        r.skip_white_spaces_and_comments();
        r.forward_tag(b"stream")?;
        r.forward_tag(b"\n").or_else(|| r.forward_tag(b"\r\n"))?;
        let data = r.read_bytes(length as usize)?;
        r.skip_white_spaces();
        r.forward_tag(b"endstream")?;

        Some(Stream { data, dict })
    }
}

impl<'a> TryFrom<Object<'a>> for Stream<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> std::result::Result<Self, Self::Error> {
        match value {
            Object::Stream(s) => Ok(s),
            _ => Err(()),
        }
    }
}

impl<'a> ObjectLike<'a> for Stream<'a> {
    const STATIC_NAME: &'static str = "Stream";
}

#[cfg(test)]
mod tests {
    use crate::file::xref::XRef;
    use crate::object::stream::Stream;
    use crate::reader::Reader;

    #[test]
    fn stream() {
        let data = b"<< /Length 10 >> stream\nabcdefghij\nendstream";
        let mut r = Reader::new(data);
        let stream = r.read_with_xref::<Stream>(&XRef::dummy()).unwrap();

        assert_eq!(stream.data, b"abcdefghij");
    }
}
