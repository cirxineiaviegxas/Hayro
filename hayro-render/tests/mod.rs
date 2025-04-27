use std::cmp::max;
use std::path::PathBuf;
use image::{load_from_memory, Rgba, RgbaImage};
use hayro_syntax::Data;
use hayro_syntax::pdf::Pdf;
use once_cell::sync::Lazy;

mod tests;

const REPLACE: Option<&str> = option_env!("REPLACE");
const STORE: Option<&str> = option_env!("STORE");

pub(crate) static WORKSPACE_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(""));

pub(crate) static ASSETS_PATH: Lazy<PathBuf> = Lazy::new(|| WORKSPACE_PATH.join("assets"));
pub(crate) static DIFFS_PATH: Lazy<PathBuf> = Lazy::new(|| WORKSPACE_PATH.join("diffs"));
pub(crate) static SNAPSHOTS_PATH: Lazy<PathBuf> = Lazy::new(|| WORKSPACE_PATH.join("snapshots"));

type RenderedDocument = Vec<Vec<u8>>;
type RenderedPage = Vec<u8>;

pub fn check_render(
    name: &str,
    document: RenderedDocument,
) {
    let mut refs_path = SNAPSHOTS_PATH.clone();

    let check_single = |name: String, page: &RenderedPage, page_num: usize| {
        let suffix = if document.len() == 1 {
            format!("{name}.png")
        }   else {
            format!("{name}_{page_num}.png")
        };
        
        let ref_path = refs_path.join(&suffix);

        if !ref_path.exists() {
            std::fs::write(&ref_path, page).unwrap();
            oxipng::optimize(
                &oxipng::InFile::Path(ref_path.clone()),
                &oxipng::OutFile::from_path(ref_path),
                &oxipng::Options::max_compression(),
            )
                .unwrap();
            panic!("new reference image was created");
        }

        let reference = load_from_memory(&std::fs::read(&ref_path).unwrap())
            .unwrap()
            .into_rgba8();
        let actual = load_from_memory(&document[page_num]).unwrap().into_rgba8();

        let (diff_image, pixel_diff) = get_diff(&reference, &actual);

        if pixel_diff > 0 {
            let diff_path = DIFFS_PATH.join(&suffix);
            diff_image
                .save_with_format(&diff_path, ::image::ImageFormat::Png)
                .unwrap();

            if REPLACE.is_some() {
                std::fs::write(&ref_path, page).unwrap();
                oxipng::optimize(
                    &oxipng::InFile::Path(ref_path.clone()),
                    &oxipng::OutFile::from_path(ref_path),
                    &oxipng::Options::max_compression(),
                )
                    .unwrap();
                panic!("test was replaced");
            }

            panic!(
                "pixel diff was {pixel_diff}"
            );
        }
    };

    if document.is_empty() {
        panic!("empty document");
    } else {
        for (index, page) in document.iter().enumerate() {
            check_single(name.to_string(), page, index);
        }
    }
}

pub fn run_test(name: &str) {
    let path = ASSETS_PATH.join(format!("{name}.pdf",));
    let content = std::fs::read(&path).unwrap();
    let data = Data::new(&content);
    let pdf = Pdf::new(&data).unwrap();
    
    check_render(name, hayro_render::render_png(&pdf, 1.0));
}

pub fn get_diff(expected_image: &RgbaImage, actual_image: &RgbaImage) -> (RgbaImage, u32) {
    let width = max(expected_image.width(), actual_image.width());
    let height = max(expected_image.height(), actual_image.height());

    let mut diff_image = RgbaImage::new(width * 3, height);

    let mut pixel_diff = 0;

    for x in 0..width {
        for y in 0..height {
            let actual_pixel = actual_image.get_pixel_checked(x, y);
            let expected_pixel = expected_image.get_pixel_checked(x, y);

            match (actual_pixel, expected_pixel) {
                (Some(actual), Some(expected)) => {
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    if is_pix_diff(expected, actual) {
                        pixel_diff += 1;
                        diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                    } else {
                        diff_image.put_pixel(x + width, y, Rgba([0, 0, 0, 255]))
                    }
                }
                (Some(actual), None) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                (None, Some(expected)) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                _ => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
            }
        }
    }

    (diff_image, pixel_diff)
}

fn is_pix_diff(pixel1: &Rgba<u8>, pixel2: &Rgba<u8>) -> bool {
    if pixel1.0[3] == 0 && pixel2.0[3] == 0 {
        return false;
    }

    pixel1.0[0] != pixel2.0[0]
        || pixel1.0[1] != pixel2.0[1]
        || pixel1.0[2] != pixel2.0[2]
        || pixel1.0[3] != pixel2.0[3]
}