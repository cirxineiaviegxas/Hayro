#[derive(Clone)]
pub struct RgbaImage {
    pub image_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub interpolate: bool,
}

#[derive(Clone)]
pub struct StencilImage {
    pub stencil_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub interpolate: bool,
}
