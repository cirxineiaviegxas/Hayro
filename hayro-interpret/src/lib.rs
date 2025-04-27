use crate::convert::{convert_line_cap, convert_line_join};
use crate::device::Device;
use hayro_syntax::content::ops::{LineCap, LineJoin, TypedOperation};
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::EXT_G_STATE;
use hayro_syntax::object::name::Name;
use hayro_syntax::object::number::Number;
use kurbo::{Cap, Join, Point, Rect, Shape};
use log::warn;
use once_cell::sync::Lazy;
use peniko::Fill;
use qcms::Transform;
use smallvec::{SmallVec, smallvec};

pub mod color;
mod convert;
pub mod device;
mod state;
mod util;

use crate::color::{Color, ColorSpace};
use crate::util::OptionLog;
pub use state::GraphicsState;

static CMYK_TRANSFORM: Lazy<Transform> = Lazy::new(|| {
    let input = qcms::Profile::new_from_slice(
        include_bytes!("../../assets/CGATS001Compat-v2-micro.icc"),
        false,
    )
    .unwrap();
    let mut output = qcms::Profile::new_sRGB();
    output.precache_output_transform();

    Transform::new_to(
        &input,
        &output,
        qcms::DataType::CMYK,
        qcms::DataType::RGB8,
        qcms::Intent::default(),
    )
    .unwrap()
});

pub struct StrokeProps {
    pub line_width: f32,
    pub line_cap: Cap,
    pub line_join: Join,
    pub miter_limit: f32,
    pub dash_array: SmallVec<[f32; 4]>,
    pub dash_offset: f32,
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
                state.get_mut().stroke_cs = ColorSpace::DeviceRgb;
                state.get_mut().stroke_color = smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32()];
            }
            TypedOperation::StrokeColorDeviceGray(s) => {
                state.get_mut().stroke_cs = ColorSpace::DeviceGray;
                state.get_mut().stroke_color = smallvec![s.0.as_f32()];
            }
            TypedOperation::StrokeColorCmyk(s) => {
                state.get_mut().stroke_cs = ColorSpace::DeviceCmyk;
                state.get_mut().stroke_color =
                    smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32(), s.3.as_f32()];
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
                let p = Point::new(m.0.as_f64(), m.1.as_f64());
                *(state.last_point_mut()) = p;
                *(state.sub_path_start_mut()) = p;
                state.path_mut().move_to(p);
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
                fill_stroke_path(state, device);
            }
            TypedOperation::FillAndStrokeNonZero(_) => {
                state.get_mut().fill = Fill::NonZero;
                fill_stroke_path(state, device);
            }
            TypedOperation::CloseAndStrokePath(_) => {
                state.path_mut().close_path();
                stroke_path(state, device);
            }
            TypedOperation::CloseFillAndStrokeEvenOdd(_) => {
                state.path_mut().close_path();
                state.get_mut().fill = Fill::EvenOdd;
                fill_stroke_path(state, device);
            }
            TypedOperation::CloseFillAndStrokeNonZero(_) => {
                state.path_mut().close_path();
                state.get_mut().fill = Fill::NonZero;
                fill_stroke_path(state, device);
            }
            TypedOperation::NonStrokeColorDeviceGray(s) => {
                state.get_mut().fill_cs = ColorSpace::DeviceGray;
                state.get_mut().fill_color = smallvec![s.0.as_f32()];
            }
            TypedOperation::NonStrokeColorDeviceRgb(s) => {
                state.get_mut().fill_cs = ColorSpace::DeviceRgb;
                state.get_mut().fill_color = smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32()];
            }
            TypedOperation::NonStrokeColorCmyk(s) => {
                state.get_mut().fill_cs = ColorSpace::DeviceCmyk;
                state.get_mut().fill_color =
                    smallvec![s.0.as_f32(), s.1.as_f32(), s.2.as_f32(), s.3.as_f32()];
            }
            TypedOperation::LineTo(m) => {
                let last_point = *state.last_point();
                let mut p = Point::new(m.0.as_f64(), m.1.as_f64());
                *(state.last_point_mut()) = p;
                if last_point == p {
                    // Add a small delta so that zero width lines can still have a round stroke.
                    p.x += 0.0001;
                }

                state.path_mut().line_to(p);
            }
            TypedOperation::CubicTo(c) => {
                let p1 = Point::new(c.0.as_f64(), c.1.as_f64());
                let p2 = Point::new(c.2.as_f64(), c.3.as_f64());
                let p3 = Point::new(c.4.as_f64(), c.5.as_f64());

                *(state.last_point_mut()) = p3;

                state.path_mut().curve_to(p1, p2, p3)
            }
            TypedOperation::CubicStartTo(c) => {
                let p1 = *state.last_point();
                let p2 = Point::new(c.0.as_f64(), c.1.as_f64());
                let p3 = Point::new(c.2.as_f64(), c.3.as_f64());

                *(state.last_point_mut()) = p3;

                state.path_mut().curve_to(p1, p2, p3)
            }
            TypedOperation::CubicEndTo(c) => {
                let p2 = Point::new(c.0.as_f64(), c.1.as_f64());
                let p3 = Point::new(c.2.as_f64(), c.3.as_f64());

                *(state.last_point_mut()) = p3;

                state.path_mut().curve_to(p2, p3, p3)
            }
            TypedOperation::ClosePath(_) => {
                state.path_mut().close_path();

                *(state.last_point_mut()) = *state.sub_path_start();
            }
            TypedOperation::SetGraphicsState(gs) => {
                let gs = ext_g_stages
                    .get::<Dict>(gs.0)
                    .warn_none(&format!("failed to get extgstate {}", gs.0.as_str()))
                    .unwrap_or_default();

                handle_gs(&gs, state);
            }
            TypedOperation::StrokePath(_) => {
                stroke_path(state, device);
            }
            TypedOperation::EndPath(_) => {
                if let Some(clip) = *state.clip() {
                    device.set_transform(state.get().affine);
                    device.push_clip(state.path(), clip);

                    *(state.clip_mut()) = None;
                    state.get_mut().n_clips += 1;
                }
                state.path_mut().truncate(0);
            }
            TypedOperation::NonStrokeColor(c) => {
                let fill_c = &mut state.get_mut().fill_color;
                fill_c.truncate(0);

                for e in c.0 {
                    fill_c.push(e.as_f32());
                }
            }
            TypedOperation::StrokeColor(c) => {
                let stroke_c = &mut state.get_mut().stroke_color;
                stroke_c.truncate(0);

                for e in c.0 {
                    stroke_c.push(e.as_f32());
                }
            }
            TypedOperation::ClipNonZero(_) => {
                *(state.clip_mut()) = Some(Fill::NonZero);
            }
            TypedOperation::ClipEvenOdd(_) => {
                *(state.clip_mut()) = Some(Fill::EvenOdd);
            }
            TypedOperation::RestoreState(_) => {
                let mut num_clips = state.get().n_clips;
                state.restore_state();
                let target_clips = state.get().n_clips;

                while num_clips > target_clips {
                    device.pop_clip();
                    num_clips -= 1;
                }
            }
            TypedOperation::FlatnessTolerance(_) => {
                // Ignore for now.
            }
            TypedOperation::ColorSpaceStroke(c) => {
                state.get_mut().stroke_cs = handle_cs(c.0);
            }
            TypedOperation::ColorSpaceNonStroke(c) => {
                state.get_mut().fill_cs = handle_cs(c.0);
            }
            TypedOperation::DashPattern(p) => {
                state.get_mut().dash_offset = p.1.as_f32();
                state.get_mut().dash_array = p.0.iter::<f32>().collect();
            }
            TypedOperation::RenderingIntent(_) => {
                // Ignore for now.
            }
            TypedOperation::NonStrokeColorNamed(n) => {
                if n.1.is_none() {
                    state.get_mut().fill_color = n.0.into_iter().map(|n| n.as_f32()).collect();
                } else {
                    warn!("named color spaces are not supported!");
                }
            }
            TypedOperation::StrokeColorNamed(n) => {
                if n.1.is_none() {
                    state.get_mut().stroke_color = n.0.into_iter().map(|n| n.as_f32()).collect();
                } else {
                    warn!("named color spaces are not supported!");
                }
            }
            TypedOperation::BeginMarkedContentWithProperties(_) => {}
            TypedOperation::MarkedContentPointWithProperties(_) => {}
            TypedOperation::EndMarkedContent(_) => {}
            TypedOperation::MarkedContentPoint(_) => {}
            TypedOperation::BeginMarkedContent(_) => {}
            _ => {
                println!("{:?}", op);
            }
        }
    }

    for _ in 0..state.get().n_clips {
        device.pop_clip();
    }
}

fn handle_cs(key: Name) -> ColorSpace {
    match key.get().as_ref() {
        b"DeviceRGB" => ColorSpace::DeviceRgb,
        b"DeviceGray" => ColorSpace::DeviceGray,
        b"DeviceCMYK" => ColorSpace::DeviceCmyk,
        _ => {
            warn!("unsupported color space {}", key.as_str());

            ColorSpace::DeviceGray
        }
    }
}

fn handle_gs(dict: &Dict, state: &mut GraphicsState) {
    for key in dict.keys() {
        handle_gs_single(dict, *key, state).warn_none(&format!(
            "invalid value in graphics state for {}",
            key.as_str()
        ));
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
    fill_path_impl(state, device);
    // TODO: Where in spec?
    state.path_mut().truncate(0);
}

fn stroke_path(state: &mut GraphicsState, device: &mut impl Device) {
    stroke_path_impl(state, device);
    state.path_mut().truncate(0);
}

fn fill_stroke_path(state: &mut GraphicsState, device: &mut impl Device) {
    fill_path_impl(state, device);
    stroke_path_impl(state, device);
    state.path_mut().truncate(0);
}

fn fill_path_impl(state: &mut GraphicsState, device: &mut impl Device) {
    let color = Color::from_pdf(
        state.get().fill_cs,
        &state.get().fill_color,
        state.get().fill_alpha,
    );
    device.set_paint(color);
    device.set_transform(state.get().affine);
    device.fill_path(state.path(), &state.fill_props());
}

fn stroke_path_impl(state: &mut GraphicsState, device: &mut impl Device) {
    let color = Color::from_pdf(
        state.get().stroke_cs,
        &state.get().stroke_color,
        state.get().stroke_alpha,
    );
    device.set_paint(color);
    device.set_transform(state.get().affine);
    device.stroke_path(state.path(), &state.stroke_props());
}
