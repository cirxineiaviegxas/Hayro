use crate::shading::Shading;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BBOX, EXT_G_STATE, MATRIX, SHADING, X_STEP, Y_STEP};
use hayro_syntax::object::{Object, dict_or_stream};
use kurbo::Affine;
use log::warn;
use std::sync::Arc;
use hayro_syntax::object::rect::Rect;
use hayro_syntax::object::stream::Stream;

#[derive(Debug, Clone)]
pub enum Pattern<'a> {
    Shading(ShadingPattern),
    Tiling(TilingPattern<'a>)
}

impl<'a> Pattern<'a> {
    pub fn new(object: Object<'a>) -> Option<Self> {
        if let Some(dict) = object.clone().into_dict() {
            Some(Self::Shading(ShadingPattern::new(&dict)?))
        }   else if let Some(stream) = object.clone().into_stream() {
            Some(Self::Tiling(TilingPattern::new(stream)?))
        }   else { 
            None
        }
    }
}

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
            .map(Affine::new)
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

#[derive(Debug, Clone)]
pub struct TilingPattern<'a> {
    pub bbox: Rect,
    pub x_step: f32,
    pub y_step: f32,
    pub matrix: Affine,
    stream: Stream<'a>,
}

impl<'a> TilingPattern<'a> {
    pub fn new(stream: Stream<'a>) -> Option<Self> {
        let dict = stream.dict();
        
        let bbox = dict.get::<Rect>(BBOX)?;
        let x_step = dict.get::<f32>(X_STEP)?;
        let y_step = dict.get::<f32>(Y_STEP)?;
        let matrix = dict
            .get::<[f64; 6]>(MATRIX)
            .map(Affine::new)
            .unwrap_or_default();
        
        Some(Self {
            bbox,
            x_step,
            y_step,
            matrix,
            stream,
        })
    }
}