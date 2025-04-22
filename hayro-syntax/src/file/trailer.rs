use crate::file::xref::{SubsectionHeader, XREF_ENTRY_LEN, XRef};
use crate::object::ObjectIdentifier;
use crate::object::dict::Dict;
use crate::object::indirect::IndirectObject;
use crate::object::stream::Stream;
use crate::reader::Reader;

pub(super) fn trailer_impl<'a>(data: &'a [u8], pos: usize, xref: &XRef<'a>) -> Option<Dict<'a>> {
    let mut reader = Reader::new(data);
    reader.jump(pos);

    if reader.clone().read_plain::<ObjectIdentifier>().is_some() {
        read_xref_stream_trailer(&mut reader, xref)
    } else {
        read_xref_table_trailer(&mut reader, xref)
    }
}

pub(super) fn read_xref_table_trailer<'a>(
    reader: &mut Reader<'a>,
    xref: &XRef<'a>,
) -> Option<Dict<'a>> {
    reader.skip_white_spaces();
    reader.forward_tag(b"xref")?;
    reader.skip_white_spaces();

    while let Some(header) = reader.read_plain::<SubsectionHeader>() {
        reader.jump(reader.offset() + XREF_ENTRY_LEN * header.num_entries as usize);
    }

    reader.skip_white_spaces();
    reader.forward_tag(b"trailer")?;
    reader.skip_white_spaces();

    reader.read_non_plain::<Dict>(xref)
}

pub(super) fn read_xref_stream_trailer<'a>(
    reader: &mut Reader<'a>,
    xref: &XRef<'a>,
) -> Option<Dict<'a>> {
    let stream = reader.read_non_plain::<IndirectObject<Stream>>(xref)?.get();

    Some(stream.dict.clone())
}
