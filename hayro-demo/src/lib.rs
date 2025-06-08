use console_error_panic_hook;
use hayro_syntax::pdf::Pdf;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

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
        Self {
            pdf: None,
            current_page: 0,
            total_pages: 0,
        }
    }

    #[wasm_bindgen]
    pub fn load_pdf(&mut self, data: &[u8]) -> Result<(), JsValue> {
        // Store the data
        let pdf = Pdf::new(Arc::new(data.to_vec())).unwrap();
        self.total_pages = pdf.pages().unwrap().len();
        self.pdf = Some(pdf);

        self.current_page = 0;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn render_current_page(&self, scale: f32) -> Result<Vec<u8>, JsValue> {
        // TODO: This could be optimized using yoke to cache the parsed PDF structure
        // instead of reparsing on every render call
        if let Some(pdf) = &self.pdf {
            let pages = pdf.pages().unwrap();

            if self.current_page < pages.len() {
                let pixmaps = hayro_render::render_png(
                    &pdf,
                    scale,
                    Some(self.current_page..=self.current_page),
                );

                if let Some(png_data) = pixmaps.first() {
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
    pub fn get_current_page(&self) -> usize {
        self.current_page + 1
    }

    #[wasm_bindgen]
    pub fn get_page_count(&self) -> usize {
        self.total_pages
    }

    #[wasm_bindgen]
    pub fn get_total_pages(&self) -> usize {
        self.total_pages
    }
}
