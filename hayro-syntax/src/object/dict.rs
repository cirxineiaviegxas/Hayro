use crate::file::xref::XRef;
use crate::object;
use crate::object::name::Name;
use crate::object::null::Null;
use crate::object::r#ref::MaybeRef;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader, Skippable};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// A dictionary, which is a key-value map, keys being names and values being any direct PDF
/// objects.
#[derive(Clone)]
pub struct Dict<'a>(Arc<Repr<'a>>);

// TODO: Is this alright to do?
impl PartialEq for Dict<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.data == other.0.data
    }
}

impl<'a> Dict<'a> {
    /// Returns the number of entries in the dictionary.
    pub fn len(&self) -> usize {
        self.0.offsets.len()
    }

    /// Checks whether the dictionary contains an entry with a specific key.
    pub fn contains_key(&self, key: Name) -> bool {
        self.0.offsets.contains_key(&key)
    }

    /// Returns the entry of a key as a specific type, and resolve it in case it's an object reference.
    pub fn get<T>(&self, key: Name) -> Option<T>
    where
        T: ObjectLike<'a>,
    {
        self.get_raw::<T>(key)?.resolve(&self.0.xref)
    }

    /// Returns an iterator over all keys in the dictionary.
    pub fn keys(&self) -> impl IntoIterator<Item = &Name> {
        self.0.offsets.keys()
    }

    pub(crate) fn get_raw<T>(&self, key: Name) -> Option<MaybeRef<T>>
    where
        T: ObjectLike<'a>,
    {
        let offset = *self.0.offsets.get(&key)?;

        Reader::new(&self.0.data[offset..]).read_non_plain::<MaybeRef<T>>(&self.0.xref)
    }
}

impl Debug for Dict<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut r = Reader::new(self.0.data);
        let mut debug_struct = f.debug_struct("Dict");

        for (key, val) in &self.0.offsets {
            r.jump(*val);
            debug_struct.field(
                &format!("{:?}", key.as_str()),
                &r.read_non_plain::<MaybeRef<Object>>(&XRef::dummy())
                    .unwrap(),
            );
        }
        Ok(())
    }
}

impl Skippable for Dict<'_> {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.forward_tag(b"<<")?;

        loop {
            r.skip_white_spaces_and_comments();

            if let Some(()) = r.forward_tag(b">>") {
                break Some(());
            } else {
                r.skip::<PLAIN, Name>()?;
                r.skip_white_spaces_and_comments();

                if PLAIN {
                    r.skip::<PLAIN, Object>()?;
                } else {
                    r.skip::<PLAIN, MaybeRef<Object>>()?;
                }
            }
        }
    }
}

impl<'a> Readable<'a> for Dict<'a> {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<Self> {
        let mut offsets = HashMap::new();

        let data = {
            let dict_data = r.tail()?;
            let start_offset = r.offset();

            r.forward_tag(b"<<")?;

            loop {
                r.skip_white_spaces_and_comments();

                if let Some(()) = r.forward_tag(b">>") {
                    let end_offset = r.offset() - start_offset;
                    break &dict_data[0..end_offset];
                } else {
                    let name = r.read_plain::<Name>()?;
                    r.skip_white_spaces_and_comments();

                    // Keys with null-objects should be treated as non-existing.
                    let is_null = {
                        let mut nr = Reader::new(r.tail()?);

                        if PLAIN {
                            nr.read_non_plain::<Null>(xref)
                        } else {
                            nr.read_non_plain::<MaybeRef<Null>>(xref)
                                .and_then(|n| n.resolve(xref))
                        }
                        .is_some()
                    };

                    if !is_null {
                        let offset = r.offset() - start_offset;
                        offsets.insert(name, offset);
                    }

                    if PLAIN {
                        r.skip::<PLAIN, Object>()?;
                    } else {
                        r.skip::<PLAIN, MaybeRef<Object>>()?;
                    }
                }
            }
        };

        Some(Dict(Arc::new(Repr {
            data,
            offsets,
            xref: xref.clone(),
        })))
    }
}

object!(Dict<'a>, Dict);

struct Repr<'a> {
    data: &'a [u8],
    offsets: HashMap<Name<'a>, usize>,
    xref: XRef<'a>,
}

#[cfg(test)]
mod tests {
    use crate::file::xref::XRef;
    use crate::object::dict::Dict;
    use crate::object::name::Name;
    use crate::object::number::Number;
    use crate::object::string;
    use crate::reader::Reader;

    fn dict_impl(data: &[u8]) -> Option<Dict> {
        Reader::new(data).read_non_plain::<Dict>(&XRef::dummy())
    }

    #[test]
    fn empty_dict_1() {
        let dict_data = b"<<>>";
        let dict = dict_impl(dict_data).unwrap();

        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn empty_dict_2() {
        let dict_data = b"<<   \n >>";
        let dict = dict_impl(dict_data).unwrap();

        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn dict_1() {
        let dict_data = b"<<  /Hi 34.0 >>";
        let dict = dict_impl(dict_data).unwrap();

        assert_eq!(dict.len(), 1);
        assert!(dict.get::<Number>(Name::from_unescaped(b"Hi")).is_some());
    }

    #[test]
    fn dict_2() {
        let dict_data = b"<<  /Hi \n 34.0 /Second true >>";
        let dict = dict_impl(dict_data).unwrap();

        assert_eq!(dict.len(), 2);
        assert!(dict.get::<Number>(Name::from_unescaped(b"Hi")).is_some());
        assert!(dict.get::<bool>(Name::from_unescaped(b"Second")).is_some());
    }

    #[test]
    fn dict_with_null() {
        let dict_data = b"<<  /Entry null /Second (Hi) >>";
        let dict = dict_impl(dict_data).unwrap();

        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn dict_complex() {
        let data = "<< /Type /Example
/Subtype /DictionaryExample
/Version 0.01
/IntegerItem 12
/StringItem ( a string )
/Subdictionary << /Item1 0.4
                /Item2 true
                /LastItem ( not ! )
                /VeryLastItem ( OK )
                >>
>>";

        let dict = Reader::new(data.as_bytes())
            .read_non_plain::<Dict>(&XRef::dummy())
            .unwrap();
        assert_eq!(dict.len(), 6);
        assert!(dict.get::<Name>(Name::from_unescaped(b"Type")).is_some());
        assert!(dict.get::<Name>(Name::from_unescaped(b"Subtype")).is_some());
        assert!(
            dict.get::<Number>(Name::from_unescaped(b"Version"))
                .is_some()
        );
        assert!(
            dict.get::<i32>(Name::from_unescaped(b"IntegerItem"))
                .is_some()
        );
        assert!(
            dict.get::<string::String>(Name::from_unescaped(b"StringItem"))
                .is_some()
        );
        assert!(
            dict.get::<Dict>(Name::from_unescaped(b"Subdictionary"))
                .is_some()
        );
    }

    #[test]
    fn dict_with_trailing() {
        let dict_data = b"<<  /Hi 67.0  >>trailing data";
        let dict = dict_impl(dict_data).unwrap();

        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn dict_with_comment() {
        let dict_data = b"<<  /Hi % A comment \n 67.0 % Another comment \n >>";
        let dict = dict_impl(dict_data).unwrap();

        assert_eq!(dict.len(), 1);
    }
}

/// A collection of possible keys in a PDF dictionary.
pub mod keys {
    use crate::object::Name;

    macro_rules! key {
        ($i:ident, $e:expr) => {
            pub const $i: Name<'static> = Name::from_unescaped($e);
        };
    }

    key!(BITS_PER_COMPONENT, b"BitsPerComponent");
    key!(COLORS, b"Colors");
    key!(COLUMNS, b"Columns");
    key!(COUNT, b"Count");
    key!(CONTENTS, b"Contents");
    key!(DECODE_PARMS, b"DecodeParms");
    key!(DOMAIN, b"Domain");
    key!(ENCRYPT, b"Encrypt");
    key!(EARLY_CHANGE, b"EarlyChange");
    key!(F, b"F");
    key!(FILTER, b"Filter");
    key!(FIRST, b"First");
    key!(INDEX, b"Index");
    key!(KIDS, b"Kids");
    key!(LENGTH, b"Length");
    key!(N, b"N");
    key!(PARENT, b"Parent");
    key!(PAGES, b"Pages");
    key!(PREDICTOR, b"Predictor");
    key!(PREV, b"Prev");
    key!(RANGE, b"Range");
    key!(ROOT, b"Root");
    key!(TYPE, b"Type");
    key!(SIZE, b"Size");
    key!(W, b"W");
}
