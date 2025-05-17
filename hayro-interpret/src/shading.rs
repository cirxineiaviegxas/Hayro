use std::sync::Arc;
use kurbo::Affine;
use crate::color::ColorSpace;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BACKGROUND, BBOX, COLORSPACE, DOMAIN, FUNCTION, MATRIX, SHADING_TYPE};
use hayro_syntax::object::rect::Rect;
use smallvec::SmallVec;
use hayro_syntax::function::Function;
use hayro_syntax::object::Object;

#[derive(Debug)]
enum ShadingType {
    FunctionBased {
        domain: [f32; 4],
        matrix: Affine,
        function: Function
    },
    Axial,
    Radial,
    FreeFormGouraud,
    LatticeFormGouraud,
    CoonsPatchMesh,
    TensorProductPatchMesh,
}

#[derive(Clone, Debug)]
pub struct Shading {
    shading_type: Arc<ShadingType>,
    color_space: ColorSpace,
    bbox: Option<Rect>,
    background: Option<SmallVec<[f32; 4]>>,
}

impl Shading {
    pub fn new(dict: &Dict) -> Option<Self> {
        let shading_type = match dict.get::<u8>(SHADING_TYPE)? {
            1 => {
                let domain = dict.get::<[f32; 4]>(DOMAIN).unwrap_or([0.0, 1.0, 0.0, 1.0]);
                let matrix = dict.get::<[f64; 6]>(MATRIX).map(|f| Affine::new(f)).unwrap_or_default();
                let function = dict.get::<Object>(FUNCTION).and_then(|f| Function::new(&f))?;
                ShadingType::FunctionBased {
                    domain,
                    matrix, function
                }
            },
            2 => ShadingType::Axial,
            3 => ShadingType::Radial,
            4 => ShadingType::FreeFormGouraud,
            5 => ShadingType::LatticeFormGouraud,
            6 => ShadingType::CoonsPatchMesh,
            7 => ShadingType::TensorProductPatchMesh,
            _ => return None,
        };

        let color_space = ColorSpace::new(dict.get(COLORSPACE)?);
        let bbox = dict.get::<Rect>(BBOX);
        let background = dict
            .get::<Array>(BACKGROUND)
            .map(|a| a.iter::<f32>().collect::<SmallVec<_>>());

        Some(Self {
            shading_type: Arc::new(shading_type),
            color_space,
            bbox,
            background,
        })
    }
}
