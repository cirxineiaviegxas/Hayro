use crate::file::xref::{SubsectionHeader, XREF_ENTRY_LEN, XRef, find_last_xref_pos};
use crate::object::ObjectIdentifier;
use crate::object::dict::Dict;
use crate::object::indirect::IndirectObject;
use crate::object::stream::Stream;
use crate::reader::Reader;

pub(crate) fn root_trailer_pos(data: &[u8]) -> Option<usize> {
    find_last_xref_pos(data)
}

pub(crate) fn read_trailer<'a>(data: &'a [u8], pos: usize, xref: &'a XRef) -> Option<Dict<'a>> {
    let mut reader = Reader::new(data);
    reader.jump(pos);

    if reader
        .clone()
        .read_without_xref::<ObjectIdentifier>()
        .is_some()
    {
        read_xref_stream_trailer(&mut reader, xref)
    } else {
        read_xref_table_trailer(&mut reader, xref)
    }
}

pub(super) fn read_xref_table_trailer<'a>(
    reader: &mut Reader<'a>,
    xref: &'a XRef,
) -> Option<Dict<'a>> {
    reader.skip_white_spaces();
    reader.forward_tag(b"xref")?;
    reader.skip_white_spaces();

    while let Some(header) = reader.read_without_xref::<SubsectionHeader>() {
        reader.jump(reader.offset() + XREF_ENTRY_LEN * header.num_entries as usize);
    }

    reader.skip_white_spaces();
    reader.forward_tag(b"trailer")?;
    reader.skip_white_spaces();

    reader.read_with_xref::<Dict>(xref)
}

fn read_xref_stream_trailer<'a>(reader: &mut Reader<'a>, xref: &'a XRef) -> Option<Dict<'a>> {
    let stream = reader.read_with_xref::<IndirectObject<Stream>>(xref)?.get();

    Some(stream.dict.clone())
}
