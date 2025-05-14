use smallvec::SmallVec;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BACKGROUND, BBOX, COLORSPACE, SHADING_TYPE};
use hayro_syntax::object::rect::Rect;
use crate::color::ColorSpace;

#[derive(Copy, Clone, Debug)]
enum ShadingType {
    FunctionBased,
    Axial,
    Radial,
    FreeFormGouraud,
    LatticeFormGouraud,
    CoonsPatchMesh,
    TensorProductPatchMesh,
}

#[derive(Clone, Debug)]
struct CommonProperties {
    shading_type: ShadingType,
    color_space: ColorSpace,
    bbox: Option<Rect>,
    background: Option<SmallVec<[f32; 4]>>,
}

impl CommonProperties {
    fn new(dict: &Dict) -> Option<Self> {
        let shading_type = match dict.get::<u8>(SHADING_TYPE)? {
            1 => ShadingType::FunctionBased,
            2 => ShadingType::Axial,
            3 => ShadingType::Radial,
            4 => ShadingType::FreeFormGouraud,
            5 => ShadingType::LatticeFormGouraud,
            6 => ShadingType::CoonsPatchMesh,
            7 => ShadingType::TensorProductPatchMesh,
            _ => return None
        };
        
        let color_space = ColorSpace::new(dict.get(COLORSPACE)?);
        let bbox = dict.get::<Rect>(BBOX);
        let background = dict.get::<Array>(BACKGROUND).map(|a| a.iter::<f32>().collect::<SmallVec<_>>());
        
        Some(Self {
            shading_type,
            color_space,
            bbox,
            background,
        })
    }
}
