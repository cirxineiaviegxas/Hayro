use crate::shading::Shading;
use hayro_syntax::function::dict_or_stream;
use hayro_syntax::object::Object;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{EXT_G_STATE, MATRIX, SHADING};
use kurbo::Affine;
use log::warn;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ShadingPattern {
    pub shading: Arc<Shading>,
    pub matrix: Affine,
}

impl ShadingPattern {
    pub fn new(dict: &Dict) -> Option<Self> {
        let shading = dict.get::<Object>(SHADING).and_then(|o| {
            let (dict, stream) = dict_or_stream(&o)?;

            Shading::new(&dict, stream.as_ref())
        })?;
        let matrix = dict
            .get::<[f64; 6]>(MATRIX)
            .map(|f| Affine::new(f))
            .unwrap_or_default();

        if dict.contains_key(EXT_G_STATE) {
            warn!("shading patterns with ext_g_state are not supported yet");
        }

        Some(Self {
            shading: Arc::new(shading),
            matrix,
        })
    }
}
