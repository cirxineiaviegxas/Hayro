use crate::object::dict::Dict;

pub fn decode(data: &[u8], _: Option<&Dict>) -> Option<Vec<u8>> {
    // TODO: Handle the color transform attribute (also in JPEG data)
    let mut decoder = zune_jpeg::JpegDecoder::new(data);
    decoder.decode_headers().ok()?;
    decoder.decode().ok()
}
