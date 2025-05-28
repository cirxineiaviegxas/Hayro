use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct PdfViewer {
    pdf_data: Option<Vec<u8>>,
    current_page: usize,
    total_pages: usize,
}

#[wasm_bindgen]
impl PdfViewer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            pdf_data: None,
            current_page: 0,
            total_pages: 0,
        }
    }

    #[wasm_bindgen]
    pub fn load_pdf(&mut self, data: &[u8]) -> Result<(), JsValue> {
        // Store the data
        self.pdf_data = Some(data.to_vec());
        let data_ref = self.pdf_data.as_ref().unwrap();

        let data = hayro_syntax::Data::new(data_ref);
        let pdf = hayro_syntax::pdf::Pdf::new(&data)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        self.total_pages = pdf.pages().unwrap().pages.len();
        self.current_page = 0;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn render_current_page(&self, scale: f32) -> Result<Vec<u8>, JsValue> {
        // TODO: This could be optimized using yoke to cache the parsed PDF structure
        // instead of reparsing on every render call
        if let Some(pdf_data) = &self.pdf_data {
            let data = hayro_syntax::Data::new(pdf_data);
            let pdf = hayro_syntax::pdf::Pdf::new(&data)
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
            let pages = pdf.pages().unwrap();

            if self.current_page < pages.pages.len() {
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
