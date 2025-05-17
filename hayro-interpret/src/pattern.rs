use std::sync::Arc;
use kurbo::Affine;
use log::warn;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{EXT_G_STATE, MATRIX, SHADING};
use hayro_syntax::object::Object;
use crate::shading::Shading;

#[derive(Clone, Debug)]
pub struct ShadingPattern {
    shading: Arc<Shading>,
    matrix: Affine
}

impl ShadingPattern {
    pub fn new(dict: &Dict) -> Option<Self> {
        let shading = dict.get::<Dict>(SHADING).and_then(|s| Shading::new(&s))?;
        let matrix = dict.get::<[f64; 6]>(MATRIX).map(|f| Affine::new(f)).unwrap_or_default();
        
        if dict.contains_key(EXT_G_STATE) {
            warn!("shading patterns with ext_g_state are not supported yet");
        }
        
        Some(Self {
            shading: Arc::new(shading),
            matrix
        })
    }
}