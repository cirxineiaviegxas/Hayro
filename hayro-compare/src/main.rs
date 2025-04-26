use hayro_syntax::Data;
use hayro_syntax::pdf::Pdf;
use sitro::RenderOptions;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let root_dir = Path::new("/Users/lstampfl/Downloads/pdfs/batch");

    let mut entries = WalkDir::new(&root_dir)
        .into_iter()
        .flat_map(|e| e.ok().map(|f| f.path().to_path_buf()))
        .flat_map(|p| {
            if p.extension().and_then(|s: &std::ffi::OsStr| s.to_str()) == Some("pdf") {
                Some(p)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    entries.sort();

    let entries = &entries[0..80];

    render_pdfium(&entries);
    render_hayro(&entries);
}

fn render_pdfium(entries: &[PathBuf]) {
    let out_dir = Path::new("/Users/lstampfl/Programming/GitHub/hayro/hayro-compare/pdfium");
    std::fs::remove_dir_all(out_dir);
    std::fs::create_dir_all(out_dir);

    for path in entries {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let file = std::fs::read(path).unwrap();
        let pages = sitro::render_pdfium(&file, &RenderOptions::default()).unwrap();

        for (idx, page) in pages.iter().enumerate() {
            let suffix = if pages.len() == 1 {
                "".to_string()
            } else {
                format!("_{}", idx)
            };
            let out_path = out_dir.join(format!("{}{}.png", stem, suffix));
            std::fs::write(out_path, page).unwrap();
        }
    }
}

fn render_hayro(entries: &[PathBuf]) {
    let out_dir = Path::new("/Users/lstampfl/Programming/GitHub/hayro/hayro-compare/hayro");
    std::fs::remove_dir_all(out_dir);
    std::fs::create_dir_all(out_dir);

    for path in entries {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let file = std::fs::read(path).unwrap();
        let data = Data::new(&file);
        let pdf = Pdf::new(&data).unwrap();
        let pages = hayro_render::render_png(&pdf, 1.0);

        for (idx, page) in pages.iter().enumerate() {
            let suffix = if pages.len() == 1 {
                "".to_string()
            } else {
                format!("_{}", idx)
            };
            let out_path = out_dir.join(format!("{}{}.png", stem, suffix));
            std::fs::write(out_path, page).unwrap();
        }
    }
}
