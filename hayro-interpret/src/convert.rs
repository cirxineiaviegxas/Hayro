use crate::Color;
use hayro_syntax::content::ops::{LineCap, LineJoin, Transform};
use peniko::color::{AlphaColor, Srgb};

pub fn convert_transform(t: Transform) -> kurbo::Affine {
    kurbo::Affine::new([
        t.0.as_f64(),
        t.2.as_f64(),
        t.1.as_f64(),
        t.3.as_f64(),
        t.4.as_f64(),
        t.5.as_f64(),
    ])
}

pub fn convert_line_cap(lc: LineCap) -> kurbo::Cap {
    match lc.0.as_i32() {
        0 => kurbo::Cap::Butt,
        1 => kurbo::Cap::Round,
        2 => kurbo::Cap::Round,
        _ => kurbo::Cap::Butt,
    }
}

pub fn convert_line_join(lc: LineJoin) -> kurbo::Join {
    match lc.0.as_i32() {
        0 => kurbo::Join::Miter,
        1 => kurbo::Join::Round,
        2 => kurbo::Join::Bevel,
        _ => kurbo::Join::Miter,
    }
}

pub fn convert_color(c: &Color) -> AlphaColor<Srgb> {
    if c.len() == 1 {
        AlphaColor::new([c[0], c[0], c[0], 1.0])
    } else if c.len() == 3 {
        AlphaColor::new([c[0], c[1], c[2], 1.0])
    } else {
        unimplemented!()
    }
}
