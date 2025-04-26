use hayro_render::{render, render_png};
use hayro_syntax::Data;
use hayro_syntax::pdf::Pdf;

fn main() {
    let file = std::fs::read("/Users/lstampfl/Downloads/pdfs/batch/pdftc_010k_0029.pdf").unwrap();
    let data = Data::new(&file);
    let pdf = Pdf::new(&data).unwrap();

    let mut pix = render_png(&pdf);

    std::fs::write("out.png", pix).unwrap();
}
