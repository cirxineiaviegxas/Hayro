use crate::Data;
use crate::file::trailer;
use crate::object::ObjectIdentifier;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{FIRST, INDEX, N, PREV, ROOT, SIZE, W, XREF_STM};
use crate::object::indirect::IndirectObject;
use crate::object::stream::Stream;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader};
use log::{error, warn};
use rustc_hash::FxHashMap;
use std::cmp::max;
use std::iter;
use std::sync::{Arc, RwLock};

pub(crate) const XREF_ENTRY_LEN: usize = 20;

/// Parse the "root" xref from the PDF.
pub(crate) fn root_xref(data: &Data) -> Option<XRef> {
    let mut xref_map = FxHashMap::default();
    let pos = find_last_xref_pos(data.get())?;
    populate_xref_impl(data.get(), pos, &mut xref_map)?;

    let xref = XRef::new(data, xref_map, false);

    Some(xref)
}

/// Try to manually parse the PDF to build an xref table and trailer dictionary.
pub(crate) fn fallback(data: &Data) -> Option<(XRef, Dict)> {
    warn!("xref table was invalid, trying to manually build xref table");
    let (xref_map, trailer_offset) = fallback_xref_map(data);

    if let Some(trailer_offset) = trailer_offset {
        warn!("rebuild xref table with {} entries", xref_map.len());
        let xref = XRef::new(data, xref_map, true);

        let mut r = Reader::new(data.get());
        r.jump(trailer_offset);
        let dict = r.read::<false, Dict>(&xref)?;

        Some((xref, dict))
    } else {
        warn!("couldn't find trailer dictionary, failed to rebuild xref table");

        None
    }
}

pub(crate) fn fallback_xref_map(data: &Data) -> (XrefMap, Option<usize>) {
    let mut xref_map = FxHashMap::default();
    let mut trailer_offset = None;

    let mut r = Reader::new(data.get());

    loop {
        let cur_pos = r.offset();

        if let Some(obj) = r.read_without_xref::<ObjectIdentifier>() {
            xref_map.insert(obj, EntryType::Normal(cur_pos));
        } else if let Some(dict) = r.read::<false, Dict>(&XRef::dummy()) {
            if dict.contains_key(SIZE) && dict.contains_key(ROOT) {
                trailer_offset = Some(cur_pos);
            }
        } else {
            r.read_byte();
        }

        if r.at_end() {
            break;
        }
    }

    (xref_map, trailer_offset)
}

/// An xref table.
#[derive(Clone, Debug)]
pub struct XRef<'a>(Inner<'a>);

impl<'a> XRef<'a> {
    fn new(data: &'a Data, xref_map: XrefMap, repaired: bool) -> Self {
        Self(Inner::Some(Arc::new(RwLock::new(SomeRepr {
            data,
            xref_map,
            repaired,
        }))))
    }

    pub(crate) fn dummy() -> Self {
        Self(Inner::Dummy)
    }

    pub(crate) fn len(&self) -> usize {
        match &self.0 {
            Inner::Dummy => 0,
            Inner::Some(s) => s.read().unwrap().xref_map.len(),
        }
    }

    pub(crate) fn objects(&'_ self) -> impl IntoIterator<Item = Object<'a>> + '_ {
        match &self.0 {
            Inner::Dummy => unimplemented!(),
            Inner::Some(s) => iter::from_fn(move || {
                let locked = s.read().unwrap();
                let mut iter = locked.xref_map.keys();

                iter.next().and_then(|k| self.get(*k))
            }),
        }
    }

    pub(crate) fn repair(&self) {
        let Inner::Some(s) = &self.0 else {
            unreachable!();
        };

        let mut locked = s.write().unwrap();
        assert!(!locked.repaired);

        let (xref_map, _) = fallback_xref_map(locked.data);
        locked.xref_map = xref_map;
        locked.repaired = true;
    }

    pub(crate) fn get<T>(&self, id: ObjectIdentifier) -> Option<T>
    where
        T: ObjectLike<'a>,
    {
        let Inner::Some(s) = &self.0 else {
            return None;
        };

        let locked = s.read().unwrap();

        let mut r = Reader::new(locked.data.get());

        match *locked.xref_map.get(&id).or_else(|| {
            // An indirect reference to an undefined object shall not be considered an error by a PDF processor; it
            // shall be treated as a reference to the null object.
            None
        })? {
            EntryType::Normal(offset) => {
                r.jump(offset);

                if let Some(object) = r.read_with_xref::<IndirectObject<T>>(self) {
                    if object.id() == &id {
                        return Some(object.get());
                    }
                } else {
                    // There is a valid object at the offset, it's just not of the type the caller
                    // expected, which is fine.
                    if r.skip_non_plain::<IndirectObject<Object>>().is_some() {
                        return None;
                    }
                };
                
                // The xref table is broken, try to repair if not already repaired.
                if locked.repaired {
                    error!(
                        "attempt was made at repairing xref, but object {:?} still couldn't be read",
                        id
                    );

                    None
                } else {
                    warn!("broken xref, attempting to repair");

                    drop(locked);
                    self.repair();

                    // Now try reading again.
                    self.get::<T>(id)
                }
            }
            EntryType::ObjStream(id, index) => {
                // Generation number is implicitly 0.
                let id = ObjectIdentifier::new(id as i32, 0);

                let stream = self.get::<Stream>(id)?;
                let data = locked.data.get_with(id, self)?;
                let object_stream = ObjectStream::new(stream, data, self.clone())?;
                object_stream.get(index)
            }
        }
    }
}

pub(crate) fn find_last_xref_pos(data: &[u8]) -> Option<usize> {
    let mut finder = Reader::new(data);
    let mut pos = finder.len() - 1;
    finder.jump(pos);

    let needle = b"startxref";

    loop {
        if finder.forward_tag(needle).is_some() {
            finder.skip_white_spaces_and_comments();

            let offset = finder.read_without_xref::<i32>()?.try_into().ok()?;

            return Some(offset);
        }

        pos = pos.checked_sub(1)?;
        finder.jump(pos);
    }
}

/// A type of xref entry.
#[derive(Debug, PartialEq, Eq)]
enum EntryType {
    /// An indirect object that is at a specific offset in the original data.
    Normal(usize),
    /// An indirect object that is part of an object stream. First number indicates the object
    /// number of the _object stream_ (the generation number is always 0), the second number indicates
    /// the index in the object stream.
    ObjStream(u32, u32),
}

type XrefMap = FxHashMap<ObjectIdentifier, EntryType>;

/// Representation of a proper xref table.
#[derive(Debug)]
struct SomeRepr<'a> {
    xref_map: XrefMap,
    data: &'a Data,
    repaired: bool,
}

#[derive(Clone, Debug)]
enum Inner<'a> {
    /// A dummy xref table that doesn't have any entries.
    Dummy,
    /// A proper xref table.
    Some(Arc<RwLock<SomeRepr<'a>>>),
}

#[derive(Debug)]
struct XRefEntry {
    offset: usize,
    gen_number: i32,
    used: bool,
}

impl XRefEntry {
    pub(crate) fn read(data: &[u8]) -> Option<XRefEntry> {
        #[inline(always)]
        fn parse_u32(data: &[u8]) -> Option<u32> {
            let mut accum = 0;

            for byte in data {
                accum = accum * 10;

                match *byte {
                    b'0'..=b'9' => accum += (*byte - b'0') as u32,
                    _ => return None,
                }
            }

            Some(accum)
        }

        let offset = parse_u32(&data[0..10])? as usize;
        let gen_number = parse_u32(&data[11..16])? as i32;

        let used = data[17] == b'n';

        Some(Self {
            offset,
            gen_number,
            used,
        })
    }
}

fn populate_xref_impl(data: &[u8], pos: usize, xref_map: &mut XrefMap) -> Option<()> {
    let mut reader = Reader::new(data);
    reader.jump(pos);

    let mut r2 = reader.clone();
    if reader
        .clone()
        .read_without_xref::<ObjectIdentifier>()
        .is_some()
    {
        populate_from_xref_stream(data, &mut r2, xref_map)
    } else {
        populate_from_xref_table(data, &mut r2, xref_map)
    }
}

pub(super) struct SubsectionHeader {
    pub(super) start: u32,
    pub(super) num_entries: u32,
}

impl Readable<'_> for SubsectionHeader {
    fn read<const PLAIN: bool>(r: &mut Reader<'_>, _: &XRef<'_>) -> Option<Self> {
        r.skip_white_spaces();
        let start = r.read_without_xref::<u32>()?;
        r.skip_white_spaces();
        let num_entries = r.read_without_xref::<u32>()?;
        r.skip_white_spaces();

        Some(Self { start, num_entries })
    }
}

fn populate_from_xref_table<'a>(
    data: &'a [u8],
    reader: &mut Reader<'a>,
    insert_map: &mut XrefMap,
) -> Option<()> {
    let trailer = {
        let mut reader = reader.clone();
        trailer::read_xref_table_trailer(&mut reader, &XRef::dummy())?
    };

    reader.skip_white_spaces();
    reader.forward_tag(b"xref")?;
    reader.skip_white_spaces();

    let mut max_obj = 0;

    if let Some(prev) = trailer.get::<i32>(PREV) {
        // First insert the entries from any previous xref tables.
        populate_xref_impl(data, prev as usize, insert_map)?;
    }

    // In hybrid files, entries in `XRefStm` should have higher priority, therefore we insert them
    // after looking at `PREV`.
    if let Some(xref_stm) = trailer.get::<i32>(XREF_STM) {
        populate_xref_impl(data, xref_stm as usize, insert_map)?;
    }

    while let Some(header) = reader.read_without_xref::<SubsectionHeader>() {
        reader.skip_white_spaces();

        let start = header.start;
        let end = start + header.num_entries;

        for obj_number in start..end {
            max_obj = max(max_obj, obj_number);
            let bytes = reader.read_bytes(XREF_ENTRY_LEN)?;
            let entry = XRefEntry::read(bytes)?;

            // Specification says we should ignore any object number > SIZE, but probably
            // not important?
            if entry.used {
                insert_map.insert(
                    ObjectIdentifier::new(obj_number as i32, entry.gen_number),
                    EntryType::Normal(entry.offset),
                );
            }
        }
    }

    Some(())
}

fn populate_from_xref_stream<'a>(
    data: &'a [u8],
    reader: &mut Reader<'a>,
    insert_map: &mut XrefMap,
) -> Option<()> {
    let stream = reader
        .read_with_xref::<IndirectObject<Stream>>(&XRef::dummy())?
        .get();

    if let Some(prev) = stream.dict.get::<i32>(PREV) {
        // First insert the entries from any previous xref tables.
        let _ = populate_xref_impl(data, prev as usize, insert_map)?;
    }

    let size = stream.dict.get::<u32>(SIZE)?;

    let [f1_len, f2_len, f3_len] = stream.dict.get::<[u8; 3]>(W)?;

    if f2_len > size_of::<u32>() as u8 {
        error!("xref offset length is larger than the allowed limit");

        return None;
    }

    // Do such files exist?
    if f1_len != 1 {
        warn!("first field in xref stream was longer than 1");
    }

    let xref_data = stream.decoded()?;
    let mut xref_reader = Reader::new(xref_data.as_ref());

    if let Some(arr) = stream.dict.get::<Array>(INDEX) {
        let mut iter = arr.iter::<(u32, u32)>();

        while let Some((start, num_elements)) = iter.next() {
            xref_stream_subsection(
                &mut xref_reader,
                start,
                num_elements,
                f1_len,
                f2_len,
                f3_len,
                insert_map,
            )?;
        }
    } else {
        xref_stream_subsection(
            &mut xref_reader,
            0,
            size,
            f1_len,
            f2_len,
            f3_len,
            insert_map,
        )?;
    }

    Some(())
}

fn xref_stream_num<'a>(data: &[u8]) -> Option<u32> {
    Some(match data.len() {
        0 => return None,
        1 => u8::from_be(data[0]) as u32,
        2 => u16::from_be_bytes(data[0..2].try_into().ok()?) as u32,
        3 => u32::from_be_bytes([0, data[0], data[1], data[2]]),
        4 => u32::from_be_bytes(data[0..4].try_into().ok()?),
        n => {
            warn!("invalid xref stream number {}", n);

            return None;
        }
    })
}

fn xref_stream_subsection<'a>(
    xref_reader: &mut Reader<'a>,
    start: u32,
    num_elements: u32,
    f1_len: u8,
    f2_len: u8,
    f3_len: u8,
    insert_map: &mut XrefMap,
) -> Option<()> {
    for i in 0..num_elements {
        let f_type = if f1_len == 0 {
            1
        } else {
            // We assume a length of 1.
            xref_reader.read_bytes(1)?[0]
        };

        let obj_number = start + i;

        match f_type {
            // We don't care about free objects.
            0 => {
                xref_reader.skip_bytes(f2_len as usize + f3_len as usize)?;
            }
            1 => {
                let offset = if f2_len > 0 {
                    let data = xref_reader.read_bytes(f2_len as usize)?;
                    xref_stream_num(data)?
                } else {
                    0
                };

                let gen_number = if f3_len > 0 {
                    let data = xref_reader.read_bytes(f3_len as usize)?;
                    xref_stream_num(data)?
                } else {
                    0
                };

                insert_map.insert(
                    ObjectIdentifier::new(obj_number as i32, gen_number as i32),
                    EntryType::Normal(offset as usize),
                );
            }
            2 => {
                let obj_stream_number = {
                    let data = xref_reader.read_bytes(f2_len as usize)?;
                    xref_stream_num(data)?
                };
                let gen_number = 0;
                let index = if f3_len > 0 {
                    let data = xref_reader.read_bytes(f3_len as usize)?;
                    xref_stream_num(data)?
                } else {
                    0
                };

                insert_map.insert(
                    ObjectIdentifier::new(obj_number as i32, gen_number),
                    EntryType::ObjStream(obj_stream_number, index),
                );
            }
            _ => {
                warn!("xref has unknown field type {}", f_type);

                return None;
            }
        }
    }

    Some(())
}

struct ObjectStream<'a> {
    data: &'a [u8],
    xref: XRef<'a>,
    offsets: Vec<usize>,
}

impl<'a> ObjectStream<'a> {
    pub fn new(inner: Stream<'a>, data: &'a [u8], xref: XRef<'a>) -> Option<Self> {
        let num_objects = inner.dict.get::<usize>(N)?;
        let first_offset = inner.dict.get::<usize>(FIRST)?;

        let mut r = Reader::new(data.as_ref());

        let mut offsets = vec![];

        for _ in 0..num_objects {
            r.skip_white_spaces_and_comments();
            // Skip object number
            let _ = r.read_without_xref::<u32>()?;
            r.skip_white_spaces_and_comments();
            let relative_offset = r.read_without_xref::<usize>()?;
            offsets.push(first_offset + relative_offset);
        }

        Some(Self {
            data,
            xref,
            offsets,
        })
    }

    pub fn get<T>(&self, index: u32) -> Option<T>
    where
        T: ObjectLike<'a>,
    {
        let offset = *self.offsets.get(index as usize)?;
        let mut r = Reader::new(&self.data);
        r.jump(offset);

        r.read_with_xref::<T>(&self.xref)
    }
}

#[cfg(test)]
mod tests {
    use crate::Data;
    use crate::file::xref::{EntryType, Inner, root_xref};
    use crate::object::ObjectIdentifier;
    use std::sync::Arc;

    #[test]
    fn basic_xref() {
        let data = Data::new(Arc::new(
            b"
otherstuff
xref
0 9
0000000000 65535 f 
0000000016 00000 n 
0000000086 00000 n 
0000000214 00000 n 
0000000391 00000 n 
0000000527 00000 n 
0000000651 00000 n 
0000000828 00000 n 
0000000968 00000 n 
trailer
<< >>
startxref
12
%%EOF",
        ));

        let Inner::Some(s) = &root_xref(&data).unwrap().0 else {
            unreachable!()
        };
        let map = &s.read().unwrap().xref_map;

        assert_eq!(
            *map.get(&ObjectIdentifier::new(1, 0)).unwrap(),
            EntryType::Normal(16)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(2, 0)).unwrap(),
            EntryType::Normal(86)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(3, 0)).unwrap(),
            EntryType::Normal(214)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(4, 0)).unwrap(),
            EntryType::Normal(391)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(5, 0)).unwrap(),
            EntryType::Normal(527)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(6, 0)).unwrap(),
            EntryType::Normal(651)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(7, 0)).unwrap(),
            EntryType::Normal(828)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(8, 0)).unwrap(),
            EntryType::Normal(968)
        );
    }

    #[test]
    fn xref_with_free_objects() {
        let data = Data::new(Arc::new(
            b"xref
0 6
0000000003 65535 f 
0000000017 00000 n 
0000000081 00000 n 
0000000000 00007 f 
0000000331 00000 n 
0000000409 00000 n 
trailer
<<
    /Size 5
>>
startxref
0",
        ));

        let Inner::Some(s) = &root_xref(&data).unwrap().0 else {
            unreachable!()
        };
        let map = &s.read().unwrap().xref_map;

        assert_eq!(
            *map.get(&ObjectIdentifier::new(1, 0)).unwrap(),
            EntryType::Normal(17)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(2, 0)).unwrap(),
            EntryType::Normal(81)
        );
        assert_eq!(map.get(&ObjectIdentifier::new(3, 0)), None);
        assert_eq!(
            *map.get(&ObjectIdentifier::new(4, 0)).unwrap(),
            EntryType::Normal(331)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(5, 0)).unwrap(),
            EntryType::Normal(409)
        );
    }

    #[test]
    fn split_xref() {
        let data = Data::new(Arc::new(
            b"xref
0 1
0000000000 65535 f 
3 1
0000000500 00000 n 
6 1
0000000698 00000 n 
9 1
0000000373 00000 n 
trailer
<<
/Size 9
/Root 13 0 R
>>
startxref
0
%%EOF",
        ));

        let Inner::Some(s) = &root_xref(&data).unwrap().0 else {
            unreachable!()
        };
        let map = &s.read().unwrap().xref_map;

        assert_eq!(
            *map.get(&ObjectIdentifier::new(3, 0)).unwrap(),
            EntryType::Normal(500)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(6, 0)).unwrap(),
            EntryType::Normal(698)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(9, 0)).unwrap(),
            EntryType::Normal(373)
        );
    }

    #[test]
    fn split_xref_with_updates() {
        let data = Data::new(Arc::new(
            b"xref
0 1
0000000000 65535 f 
3 1
0000025325 00000 n 
23 2
0000025518 00002 n 
0000025635 00000 n 
30 1
0000025777 00000 n 
trailer
<<
    /Size 30
>>
startxref
0",
        ));

        let Inner::Some(s) = &root_xref(&data).unwrap().0 else {
            unreachable!()
        };
        let map = &s.read().unwrap().xref_map;

        assert_eq!(
            *map.get(&ObjectIdentifier::new(3, 0)).unwrap(),
            EntryType::Normal(25325)
        );
        assert_eq!(map.get(&ObjectIdentifier::new(23, 0)), None);
        assert_eq!(
            *map.get(&ObjectIdentifier::new(23, 2)).unwrap(),
            EntryType::Normal(25518)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(30, 0)).unwrap(),
            EntryType::Normal(25777)
        );
    }

    #[test]
    fn updated_xref_table() {
        let data = Data::new(Arc::new(
            b"xref
0 4
0000000000 65535 f 
0000000016 00000 n 
0000000086 00000 n 
0000000150 00000 n 
trailer
<<
    /Size 4
>>
startxref
0
%%EOF

xref
0 1
0000000000 65535 f 
2 1
0000000250 00000 n 
trailer
<<
    /Prev 0
    /Size 3
>>
startxref
134
%%EOF",
        ));

        let Inner::Some(s) = &root_xref(&data).unwrap().0 else {
            unreachable!()
        };
        let map = &s.read().unwrap().xref_map;

        assert_eq!(
            *map.get(&ObjectIdentifier::new(1, 0)).unwrap(),
            EntryType::Normal(16)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(2, 0)).unwrap(),
            EntryType::Normal(250)
        );
        assert_eq!(
            *map.get(&ObjectIdentifier::new(3, 0)).unwrap(),
            EntryType::Normal(150)
        );
    }
}
