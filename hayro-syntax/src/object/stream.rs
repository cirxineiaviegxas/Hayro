use crate::Result;
use crate::file::xref::XRef;
use crate::filter::{Filter, apply_filter};
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{DECODE_PARMS, F, FILTER, LENGTH};
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader, Skippable};
use log::warn;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};

/// A stream of arbitrary data.
#[derive(Clone)]
pub struct Stream<'a> {
    pub(crate) dict: Dict<'a>,
    data: &'a [u8],
}

impl<'a> Stream<'a> {
    /// Return the raw (potentially with some applied filters) data of the stream.
    pub fn raw_data(&self) -> &'a [u8] {
        self.data
    }

    /// Return the decoded data of the stream.
    ///
    /// Note that the result of this method will not be cached, so calling it multiple
    /// times is expensive.
    pub fn decoded(&self) -> Result<Cow<'a, [u8]>> {
        if let Some(filter) = self.dict.get::<Filter>(FILTER) {
            let params = self.dict.get::<Dict>(DECODE_PARMS);

            Ok(Cow::Owned(apply_filter(
                self.data,
                filter,
                params.as_ref(),
            )?))
        } else if let Some(filters) = self.dict.get::<Array>(FILTER) {
            // TODO: Avoid allocation?

            let filters = filters.iter::<Filter>().collect::<Vec<_>>();
            let params = self
                .dict
                .get::<Array>(DECODE_PARMS)
                .map(|a| a.iter::<Dict>().collect())
                .unwrap_or(vec![]);

            let mut current = Cow::Borrowed(self.data);

            for i in 0..filters.len() {
                let new = apply_filter(current.as_ref(), filters[i], params.get(i))?;
                current = Cow::Owned(new);
            }

            Ok(current)
        } else {
            Ok(Cow::Borrowed(self.data))
        }
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
        let dict = r.read_non_plain::<Dict>(xref)?;

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
        let stream = r.read_non_plain::<Stream>(&XRef::dummy()).unwrap();

        assert_eq!(stream.data, b"abcdefghij");
    }
}
