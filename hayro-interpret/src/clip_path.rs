use kurbo::BezPath;
use peniko::Fill;

#[derive(Debug, Clone)]
pub struct ClipPath {
    pub path: BezPath,
    pub fill: Fill,
}