use std::fmt::format;
use crate::convert::{convert_color, convert_line_cap, convert_line_join};
use crate::device::Device;
use hayro_syntax::content::ops::{LineCap, LineJoin, TypedOperation};
use kurbo::{Affine, BezPath, Cap, Join, Point, Rect, Shape, Stroke};
use peniko::Fill;
use smallvec::{SmallVec, smallvec};
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{EXT_G_STATE};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::number::Number;

type Color = SmallVec<[f32; 4]>;

mod convert;
pub mod device;
mod state;
mod util;

pub use state::GraphicsState;
use crate::util::OptionLog;

pub struct StrokeProps {
    pub line_width: f32,
    pub line_cap: Cap,
    pub line_join: Join,
    pub miter_limit: f32,
}

pub struct FillProps {
    pub fill_rule: Fill,
}

pub fn interpret<'a>(
    ops: impl Iterator<Item = TypedOperation<'a>>,
    resources: Dict,
    state: &mut GraphicsState,
    device: &mut impl Device,
) {
    let ext_g_stages = resources.get::<Dict>(EXT_G_STATE).unwrap_or_default();
    
    for op in ops {
        match op {
            TypedOperation::SaveState(_) => state.save_state(),
            TypedOperation::StrokeColorDeviceRgb(s) => {
                state.get_mut().stroke_color = smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32()];
            }
            TypedOperation::StrokeColorDeviceGray(s) => {
                state.get_mut().stroke_color = smallvec![s.0.as_f32()];
            }
            TypedOperation::LineWidth(w) => {
                state.get_mut().line_width = w.0.as_f32();
            }
            TypedOperation::LineCap(c) => {
                state.get_mut().line_cap = convert_line_cap(c);
            }
            TypedOperation::LineJoin(j) => {
                state.get_mut().line_join = convert_line_join(j);
            }
            TypedOperation::MiterLimit(l) => {
                state.get_mut().miter_limit = l.0.as_f32();
            }
            TypedOperation::Transform(t) => {
                state.pre_concat_transform(t);
            }
            TypedOperation::RectPath(r) => {
                let rect = Rect::new(
                    r.0.as_f64(),
                    r.1.as_f64(),
                    r.0.as_f64() + r.2.as_f64(),
                    r.1.as_f64() + r.3.as_f64(),
                )
                .to_path(0.1);
                state.path_mut().extend(rect);
            }
            TypedOperation::MoveTo(m) => {
                state
                    .path_mut()
                    .move_to(Point::new(m.0.as_f64(), m.1.as_f64()));
            }
            TypedOperation::FillPathEvenOdd(_) => {
                state.get_mut().fill = Fill::EvenOdd;
                fill_path(state, device);
            }
            TypedOperation::FillPathNonZero(_) => {
                state.get_mut().fill = Fill::NonZero;
                fill_path(state, device);
            }
            TypedOperation::FillPathNonZeroCompatibility(_) => {
                state.get_mut().fill = Fill::NonZero;
                fill_path(state, device);
            }
            TypedOperation::FillAndStrokeEvenOdd(_) => {
                state.get_mut().fill = Fill::EvenOdd;
                fill_path(state, device);
                stroke_path(state, device);
            }
            TypedOperation::FillAndStrokeNonZero(_) => {
                state.get_mut().fill = Fill::NonZero;
                fill_path(state, device);
                stroke_path(state, device);
            }
            TypedOperation::CloseAndStrokePath(_) => {
                state.path_mut().close_path();
                stroke_path(state, device);
            }
            TypedOperation::CloseFillAndStrokeEvenOdd(_) => {
                state.path_mut().close_path();
                state.get_mut().fill = Fill::EvenOdd;
                fill_path(state, device);
                stroke_path(state, device);
            }
            TypedOperation::CloseFillAndStrokeNonZero(_) => {
                state.path_mut().close_path();
                state.get_mut().fill = Fill::NonZero;
                fill_path(state, device);
                stroke_path(state, device);
            }
            TypedOperation::NonStrokeColorDeviceGray(d) => {
                state.get_mut().fill_color = smallvec![d.0.as_f32()];
            }
            TypedOperation::NonStrokeColorDeviceRgb(d) => {
                state.get_mut().fill_color = smallvec![d.0.as_f32(), d.1.as_f32(), d.2.as_f32()];
            }
            TypedOperation::LineTo(m) => {
                state
                    .path_mut()
                    .line_to(Point::new(m.0.as_f64(), m.1.as_f64()));
            }
            TypedOperation::CubicTo(c) => {
                state
                    .path_mut()
                    .curve_to(Point::new(c.0.as_f64(), c.1.as_f64()), Point::new(c.2.as_f64(), c.3.as_f64()), Point::new(c.4.as_f64(), c.5.as_f64()))
            }
            TypedOperation::ClosePath(_) => {
                state.path_mut().close_path();
            }
            TypedOperation::SetGraphicsState(gs) => {
                let gs = ext_g_stages.get::<Dict>(gs.0)
                    .warn_none(&format!("failed to get extgstate {}", gs.0.as_str()))
                    .unwrap_or_default();
                
                handle_gs(&gs, state);
            }
            TypedOperation::StrokePath(_) => {
                stroke_path(state, device);
            }
            TypedOperation::RestoreState(_) => state.restore_state(),
            _ => {
                println!("{:?}", op);
            }
        }
    }
}

fn handle_gs(dict: &Dict, state: &mut GraphicsState) {
    for key in dict.keys() { 
        handle_gs_single(dict, *key, state)
            .warn_none(&format!("invalid value in graphics state for {}", key.as_str()));
    }
}

fn handle_gs_single(dict: &Dict, key: Name, state: &mut GraphicsState) -> Option<()> {
    
    // TODO Can we use constants here somehow?
    match key.as_str().as_str() {
        "LW" => state.get_mut().line_width = dict.get::<f32>(key)?,
        "LC" => state.get_mut().line_cap = convert_line_cap(LineCap(dict.get::<Number>(key)?)),
        "LJ" => state.get_mut().line_join = convert_line_join(LineJoin(dict.get::<Number>(key)?)),
        "ML" => state.get_mut().miter_limit = dict.get::<f32>(key)?,
        "CA" => state.get_mut().stroke_alpha = dict.get::<f32>(key)?,
        "ca" => state.get_mut().fill_alpha = dict.get::<f32>(key)?,
        "Type" => {}
        _ => {}
    }
    
    Some(())
}

fn fill_path(state: &mut GraphicsState, device: &mut impl Device) {
    let color = convert_color(&state.get().fill_color, state.get().fill_alpha);
    device.set_paint(color);
    device.set_transform(state.get().affine);
    device.fill_path(state.path(), &state.fill_props());

    // TODO: Where in spec?
    state.path_mut().truncate(0);
}

fn stroke_path(state: &mut GraphicsState, device: &mut impl Device) {
    let color = convert_color(&state.get().stroke_color, state.get().stroke_alpha);
    device.set_paint(color);
    device.set_transform(state.get().affine);
    device.stroke_path(state.path(), &state.stroke_props());

    state.path_mut().truncate(0);
}
