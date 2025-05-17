use crate::color::ColorSpace;
use hayro_syntax::function::Function;
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{
    BACKGROUND, BBOX, COLORSPACE, COORDS, DOMAIN, EXTEND, FUNCTION, MATRIX, SHADING_TYPE,
};
use hayro_syntax::object::rect::Rect;
use kurbo::Affine;
use smallvec::SmallVec;
use std::sync::Arc;

#[derive(Debug)]
pub enum ShadingType {
    FunctionBased {
        domain: [f32; 4],
        matrix: Affine,
        function: Function,
    },
    RadialAxial {
        coords: [f32; 6],
        domain: [f32; 2],
        function: Function,
        extend: [bool; 2],
        axial: bool,
    },
    FreeFormGouraud,
    LatticeFormGouraud,
    CoonsPatchMesh,
    TensorProductPatchMesh,
}

#[derive(Clone, Debug)]
pub struct Shading {
    pub shading_type: Arc<ShadingType>,
    pub color_space: ColorSpace,
    pub bbox: Option<Rect>,
    pub background: Option<SmallVec<[f32; 4]>>,
}

impl Shading {
    pub fn new(dict: &Dict) -> Option<Self> {
        let shading_num = dict.get::<u8>(SHADING_TYPE)?;
        let shading_type = match shading_num {
            1 => {
                let domain = dict.get::<[f32; 4]>(DOMAIN).unwrap_or([0.0, 1.0, 0.0, 1.0]);
                let matrix = dict
                    .get::<[f64; 6]>(MATRIX)
                    .map(|f| Affine::new(f))
                    .unwrap_or_default();
                // TODO: Array of functions is permissible as well.
                let function = dict
                    .get::<Object>(FUNCTION)
                    .and_then(|f| Function::new(&f))?;
                ShadingType::FunctionBased {
                    domain,
                    matrix,
                    function,
                }
            }
            2 | 3 => {
                let domain = dict.get::<[f32; 2]>(DOMAIN).unwrap_or([0.0, 1.0]);
                // TODO: Array of functions is permissible as well.
                let function = dict
                    .get::<Object>(FUNCTION)
                    .and_then(|f| Function::new(&f))?;
                let extend = dict.get::<[bool; 2]>(EXTEND).unwrap_or([false, false]);
                let coords = if shading_num == 2 {
                    let read = dict.get::<[f32; 4]>(COORDS)?;
                    [read[0], read[1], read[2], read[3], 0.0, 0.0]
                } else {
                    dict.get::<[f32; 6]>(COORDS)?
                };

                let axial = shading_num == 2;

                ShadingType::RadialAxial {
                    domain,
                    function,
                    extend,
                    coords,
                    axial,
                }
            }
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
