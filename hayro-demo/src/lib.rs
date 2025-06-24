use console_error_panic_hook;
use hayro_syntax::pdf::Pdf;
use js_sys;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

// Custom logger to forward Rust logs to browser console
struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::LevelFilter::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let message = format!(
                "[{}:{}] {}",
                record.target(),
                record.line().unwrap_or(0),
                record.args()
            );

            // Get level string for the log window
            let level_str = match record.level() {
                log::Level::Error => "error",
                log::Level::Warn => "warn",
                log::Level::Info => "info",
                log::Level::Debug => "debug",
                log::Level::Trace => "trace",
            };

            // Log to browser console
            match record.level() {
                log::Level::Error => web_sys::console::error_1(&message.clone().into()),
                log::Level::Warn => web_sys::console::warn_1(&message.clone().into()),
                _ => web_sys::console::log_1(&message.clone().into()),
            }

            // Also log to our custom log window if the function exists
            if let Ok(window) = web_sys::window().ok_or("no window") {
                if let Ok(add_log_entry) = js_sys::Reflect::get(&window, &"addLogEntry".into()) {
                    if add_log_entry.is_function() {
                        let function = js_sys::Function::from(add_log_entry);
                        let _ = function.call2(&window, &level_str.into(), &message.into());
                    }
                }
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: ConsoleLogger = ConsoleLogger;

#[wasm_bindgen]
pub struct PdfViewer {
    pdf: Option<Pdf>,
    current_page: usize,
    total_pages: usize,
}

#[wasm_bindgen]
impl PdfViewer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();

        // Initialize logger to forward Rust logs to browser console
        if log::set_logger(&LOGGER).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }

        Self {
            pdf: None,
            current_page: 0,
            total_pages: 0,
        }
    }

    #[wasm_bindgen]
    pub fn load_pdf(&mut self, data: &[u8]) -> Result<(), JsValue> {
        let pdf = Pdf::new(Arc::new(data.to_vec()))
            .ok_or_else(|| JsValue::from_str("Failed to parse PDF"))?;

        let pages = pdf
            .pages()
            .ok_or_else(|| JsValue::from_str("Failed to get pages"))?;

        self.total_pages = pages.len();
        self.pdf = Some(pdf);
        self.current_page = 0;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn render_current_page(&self) -> Result<Vec<u8>, JsValue> {
        if let Some(pdf) = &self.pdf {
            if self.current_page < self.total_pages {
                let pixmaps = hayro_render::render_png(
                    &pdf,
                    2.0, // Fixed scale, no zoom
                    Some(self.current_page..=self.current_page),
                );

                if let Some(png_data) = pixmaps.as_ref().and_then(|p| p.first()) {
                    return Ok(png_data.clone());
                }
            }
        }
        Err(JsValue::from_str("Failed to render page"))
    }

    #[wasm_bindgen]
    pub fn next_page(&mut self) -> bool {
        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
            true
        } else {
            false
        }
    }

    #[wasm_bindgen]
    pub fn previous_page(&mut self) -> bool {
        if self.current_page > 0 {
            self.current_page -= 1;
            true
        } else {
            false
        }
    }

    #[wasm_bindgen]
    pub fn set_page(&mut self, page: usize) -> bool {
        if page > 0 && page <= self.total_pages {
            self.current_page = page - 1; // Convert from 1-indexed to 0-indexed
            true
        } else {
            false
        }
    }

    #[wasm_bindgen]
    pub fn get_current_page(&self) -> usize {
        self.current_page + 1 // Convert to 1-indexed
    }

    #[wasm_bindgen]
    pub fn get_total_pages(&self) -> usize {
        self.total_pages
    }
}
