use crate::file::xref::XRef;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::name::Name;
use crate::object::null::Null;
use crate::object::number::Number;
use crate::object::stream::Stream;
use crate::reader::{Readable, Reader, Skippable};
use std::fmt::Debug;

pub mod array;
pub mod bool;
pub mod dict;
pub(crate) mod indirect;
pub mod name;
pub mod null;
pub mod number;
pub mod rect;
pub mod r#ref;
pub mod stream;
pub mod string;
mod tuple;

/// A trait for PDF objects.
pub(crate) trait ObjectLike<'a>: TryFrom<Object<'a>> + Readable<'a> + Debug + Clone {}

#[macro_export]
macro_rules! object {
    ($t:ident $(<$l:lifetime>),*, $s:ident) => {
        impl<'a> TryFrom<Object<'a>> for $t$(<$l>),* {
            type Error = ();

            fn try_from(value: Object<'a>) -> std::result::Result<Self, Self::Error> {
                match value {
                    Object::$s(b) => Ok(b),
                    _ => Err(()),
                }
            }
        }

        impl<'a> crate::object::ObjectLike<'a> for $t$(<$l>),* {}
    };
}

/// A primitive PDF object.
#[derive(Debug, Clone, PartialEq)]
pub enum Object<'a> {
    /// A null object.
    Null(Null),
    /// A boolean object.
    Boolean(bool),
    /// A number object.
    Number(Number),
    /// A string object.
    String(string::String<'a>),
    /// A name object.
    Name(Name<'a>),
    /// A dict object.
    Dict(Dict<'a>),
    /// An array object.
    Array(Array<'a>),
    /// A stream object.
    // Can only be an indirect object in theory and thus comes with some caveats,
    // but we just treat it the same.
    Stream(Stream<'a>),
}

impl<'a> Object<'a> {
    /// Try casting the object to a specific subtype.
    pub(crate) fn cast<T>(self) -> Option<T>
    where
        T: ObjectLike<'a>,
    {
        self.try_into().ok()
    }

    #[inline(always)]
    pub fn into_dict(self) -> Option<Dict<'a>> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_name(self) -> Option<Name<'a>> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_null(self) -> Option<Null> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_bool(self) -> Option<bool> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_string(self) -> Option<string::String<'a>> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_stream(self) -> Option<Stream<'a>> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_array(self) -> Option<Array<'a>> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_u8(self) -> Option<u8> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_u16(self) -> Option<u16> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_f32(self) -> Option<f32> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_i32(self) -> Option<i32> {
        self.cast()
    }

    #[inline(always)]
    pub fn into_number(self) -> Option<Number> {
        self.cast()
    }
}

impl<'a> ObjectLike<'a> for Object<'a> {}

impl Skippable for Object<'_> {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        match r.peek_byte()? {
            b'n' => Null::skip::<PLAIN>(r),
            b't' | b'f' => bool::skip::<PLAIN>(r),
            b'/' => Name::skip::<PLAIN>(r),
            b'<' => match r.peek_bytes(2)? {
                // A stream can never appear in a dict/array, so it should never be skipped.
                b"<<" => Dict::skip::<PLAIN>(r),
                _ => string::String::skip::<PLAIN>(r),
            },
            b'(' => string::String::skip::<PLAIN>(r),
            b'.' | b'+' | b'-' | b'0'..=b'9' => Number::skip::<PLAIN>(r),
            b'[' => Array::skip::<PLAIN>(r),
            _ => None,
        }
    }
}

impl<'a> Readable<'a> for Object<'a> {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        let object = match r.peek_byte()? {
            b'n' => Self::Null(Null::read::<PLAIN>(r, xref)?),
            b't' | b'f' => Self::Boolean(bool::read::<PLAIN>(r, xref)?),
            b'/' => Self::Name(Name::read::<PLAIN>(r, xref)?),
            b'<' => match r.peek_bytes(2)? {
                b"<<" => {
                    let mut cloned = r.clone();
                    let dict = Dict::read::<PLAIN>(&mut cloned, xref)?;
                    cloned.skip_white_spaces_and_comments();

                    if cloned.forward_tag(b"stream").is_some() {
                        Object::Stream(Stream::read::<PLAIN>(r, xref)?)
                    } else {
                        r.jump(cloned.offset());

                        Object::Dict(dict)
                    }
                }
                _ => Self::String(string::String::read::<PLAIN>(r, xref)?),
            },
            b'(' => Self::String(string::String::read::<PLAIN>(r, xref)?),
            b'.' | b'+' | b'-' | b'0'..=b'9' => Self::Number(Number::read::<PLAIN>(r, xref)?),
            b'[' => Self::Array(Array::read::<PLAIN>(r, xref)?),
            _ => return None,
        };

        Some(object)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct ObjectIdentifier {
    pub(crate) obj_num: i32,
    pub(crate) gen_num: i32,
}

impl ObjectIdentifier {
    pub(crate) fn new(obj_num: i32, gen_num: i32) -> Self {
        Self { obj_num, gen_num }
    }
}

impl Readable<'_> for ObjectIdentifier {
    fn read<const PLAIN: bool>(r: &mut Reader<'_>, _: &XRef<'_>) -> Option<Self> {
        let obj_num = r.read_without_xref::<i32>()?;
        r.skip_white_spaces_and_comments();
        let gen_num = r.read_without_xref::<i32>()?;
        r.skip_white_spaces_and_comments();
        r.forward_tag(b"obj")?;

        Some(ObjectIdentifier { obj_num, gen_num })
    }
}

/// A convenience function that extracts a dict and a stream from an object.
/// If the object is just a dictionary, it will return `None` for the stream.
/// If the object is a stream, it will return it's dictionary as well as the stream
/// itself.
pub fn dict_or_stream<'a>(obj: &Object<'a>) -> Option<(Dict<'a>, Option<Stream<'a>>)> {
    if let Some(stream) = obj.clone().cast::<Stream>() {
        Some((stream.dict().clone(), Some(stream)))
    } else if let Some(dict) = obj.clone().cast::<Dict>() {
        Some((dict, None))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::file::xref::XRef;
    use crate::object::Object;
    use crate::reader::Reader;

    fn object_impl(data: &[u8]) -> Option<Object> {
        let mut r = Reader::new(data);
        r.read_with_xref::<Object>(&XRef::dummy())
    }

    #[test]
    fn null() {
        assert!(matches!(object_impl(b"null").unwrap(), Object::Null(_)))
    }

    #[test]
    fn bool() {
        assert!(matches!(object_impl(b"true").unwrap(), Object::Boolean(_)))
    }

    #[test]
    fn number() {
        assert!(matches!(object_impl(b"34.5").unwrap(), Object::Number(_)))
    }

    #[test]
    fn string_1() {
        assert!(matches!(object_impl(b"(Hi)").unwrap(), Object::String(_)))
    }

    #[test]
    fn string_2() {
        assert!(matches!(object_impl(b"<34>").unwrap(), Object::String(_)))
    }

    #[test]
    fn name() {
        assert!(matches!(object_impl(b"/Name").unwrap(), Object::Name(_)))
    }

    #[test]
    fn dict() {
        assert!(matches!(
            object_impl(b"<</Entry 45>>").unwrap(),
            Object::Dict(_)
        ))
    }

    #[test]
    fn array() {
        assert!(matches!(object_impl(b"[45]").unwrap(), Object::Array(_)))
    }

    #[test]
    fn stream() {
        assert!(matches!(
            object_impl(b"<< /Length 3 >> stream\nabc\nendstream").unwrap(),
            Object::Stream(_)
        ))
    }
}
