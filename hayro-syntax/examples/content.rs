use hayro_syntax::Data;
use hayro_syntax::content::ops::TypedOperation;
use hayro_syntax::pdf::Pdf;
use walkdir::WalkDir;

#[allow(dead_code)]
fn main() {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }

    let root_folder = "/Users/lstampfl/Downloads/pdfs";

    let mut entries = WalkDir::new(root_folder)
        .into_iter()
        .flat_map(|e| e.ok().map(|f| f.path().to_path_buf()))
        .flat_map(|p| {
            if p.extension().and_then(|s: &std::ffi::OsStr| s.to_str()) == Some("pdf")
                && !p.as_path().to_str().is_some_and(|p| p.contains("cleaned"))
            {
                Some(p)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    entries.sort();

    for path in &entries[0..500] {
        let file = std::fs::read(path.as_path()).unwrap();
        let data = Data::new(&file);
        let pdf = Pdf::new(&data);

        if let Ok(pdf) = pdf {
            let pages = pdf.pages().unwrap();

            println!("{:?}", path);
            for page in &pages.pages {
                for op in page.typed_operations() {
                    if matches!(op, TypedOperation::Fallback) {
                        println!("{:?}", op);
                    }
                }
            }
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
