//! A decoder for JPX-encoded images.

use crate::filter::FilterResult;

/// Decode a JPX-encoded image stream.
#[cfg(feature = "jpeg2000")]
pub fn decode(data: &[u8]) -> Option<FilterResult> {
    use crate::filter::ImageColorSpace;

    let image = jpeg2k::Image::from_bytes(data).unwrap();
    let components = image.components();
    let cs = match components.len() {
        1 => Some(ImageColorSpace::Gray),
        3 => Some(ImageColorSpace::Rgb),
        4 => Some(ImageColorSpace::Cmyk),
        _ => None,
    };
    let bpc = components
        .iter()
        .fold(std::u32::MIN, |max, c| max.max(c.precision())) as u8;
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

/// A stub-method for decoding a JPX-encoded image stream. Always returns `None`.
#[cfg(not(feature = "jpeg2000"))]
pub fn decode(_: &[u8]) -> Option<FilterResult> {
    log::warn!("Support for JPEG2000 images is not supported by the current build.");

    None
}
