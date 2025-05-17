use hayro_syntax::Data;
use hayro_syntax::pdf::Pdf;
use sitro::RenderOptions;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MAX_PAGES: usize = 3;

fn main() {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }

    let root_dir = Path::new("/Users/lstampfl/Downloads/pdfs/color_space");

    let mut entries = WalkDir::new(root_dir)
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

    let entries = &entries;

    render_pdfium(entries);
    render_hayro(entries);
}

fn render_pdfium(entries: &[PathBuf]) {
    let out_dir = Path::new("/Users/lstampfl/Programming/GitHub/hayro/hayro-compare/pdfium");
    let _ = std::fs::remove_dir_all(out_dir);
    let _ = std::fs::create_dir_all(out_dir);

    for path in entries {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let file = std::fs::read(path).unwrap();
        let pages = sitro::render_pdfium(&file, &RenderOptions::default()).unwrap();

        for (idx, page) in pages.iter().enumerate().take(MAX_PAGES) {
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
    let _ = std::fs::remove_dir_all(out_dir);
    let _ = std::fs::create_dir_all(out_dir);

    for path in entries {
        println!("{}", path.display());

        let stem = path.file_stem().unwrap().to_str().unwrap();
        let file = std::fs::read(path).unwrap();
        let data = Data::new(&file);
        let pdf = Pdf::new(&data).unwrap();
        let pages = hayro_render::render_png(&pdf, 1.0, None);

        for (idx, page) in pages.iter().enumerate().take(MAX_PAGES) {
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

/// A simple stderr logger.
static LOGGER: SimpleLogger = SimpleLogger;
struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::LevelFilter::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            let line = record.line().unwrap_or(0);
            let args = record.args();

            match record.level() {
                log::Level::Error => eprintln!("Error (in {}:{}): {}", target, line, args),
                log::Level::Warn => eprintln!("Warning (in {}:{}): {}", target, line, args),
                log::Level::Info => eprintln!("Info (in {}:{}): {}", target, line, args),
                log::Level::Debug => eprintln!("Debug (in {}:{}): {}", target, line, args),
                log::Level::Trace => eprintln!("Trace (in {}:{}): {}", target, line, args),
            }
        }
    }

    fn flush(&self) {}
}
