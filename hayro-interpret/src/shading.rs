use crate::color::ColorSpace;
use hayro_syntax::bit::{BitReader, BitSize};
use hayro_syntax::function::{Function, interpolate};
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BACKGROUND, BBOX, BITS_PER_COMPONENT, BITS_PER_COORDINATE, BITS_PER_FLAG, COLORSPACE, COORDS, DECODE, DOMAIN, EXTEND, FUNCTION, MATRIX, SHADING_TYPE, VERTICES_PER_ROW};
use hayro_syntax::object::rect::Rect;
use hayro_syntax::object::stream::Stream;
use hayro_syntax::reader::Reader;
use kurbo::{Affine, Point};
use smallvec::{SmallVec, smallvec};
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
    TriangleMesh {
        triangles: Vec<Triangle>,
        function: Option<Function>,
    },
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
    pub fn new(dict: &Dict, stream: Option<&Stream>) -> Option<Self> {
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
            4 => {
                let stream = stream?;
                let stream_data = stream.decoded().ok()?;
                let bp_coord = dict.get::<u8>(BITS_PER_COORDINATE)?;
                let bp_comp = dict.get::<u8>(BITS_PER_COMPONENT)?;
                let bpf = dict.get::<u8>(BITS_PER_FLAG)?;
                let function = dict.get::<Object>(FUNCTION).and_then(|o| Function::new(&o));
                let decode = dict.get::<Array>(DECODE)?.iter::<f32>().collect::<Vec<_>>();

                let triangles = read_free_form_triangles(
                    stream_data.as_ref(),
                    bpf,
                    bp_coord,
                    bp_comp,
                    function.as_ref(),
                    &decode,
                )?;

                ShadingType::TriangleMesh {
                    triangles,
                    function,
                }
            }
            5 => {
                let stream = stream?;
                let stream_data = stream.decoded().ok()?;
                let bp_coord = dict.get::<u8>(BITS_PER_COORDINATE)?;
                let bp_comp = dict.get::<u8>(BITS_PER_COMPONENT)?;
                let function = dict.get::<Object>(FUNCTION).and_then(|o| Function::new(&o));
                let decode = dict.get::<Array>(DECODE)?.iter::<f32>().collect::<Vec<_>>();
                let vertices_per_row = dict.get::<u32>(VERTICES_PER_ROW)?;

                let triangles = read_lattice_triangles(
                    stream_data.as_ref(),
                    bp_coord,
                    bp_comp,
                    function.as_ref(),
                    vertices_per_row,
                    &decode,
                )?;

                ShadingType::TriangleMesh {
                    triangles,
                    function,
                }
            }
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

#[derive(Clone, Debug)]
pub struct Triangle {
    pub p0: TriangleVertex,
    pub p1: TriangleVertex,
    pub p2: TriangleVertex,
}

#[derive(Clone, Debug)]
pub struct TriangleVertex {
    flag: u32,
    pub x: f32,
    pub y: f32,
    pub colors: Vec<f32>,
}

fn read_free_form_triangles(
    data: &[u8],
    bpf: u8,
    bp_cord: u8,
    bp_comp: u8,
    function: Option<&Function>,
    decode: &[f32],
) -> Option<Vec<Triangle>> {
    let bpf = BitSize::from_u8(bpf)?;
    let bp_cord = BitSize::from_u8(bp_cord)?;
    let bp_comp = BitSize::from_u8(bp_comp)?;

    let mut triangles = vec![];

    let ([x_min, x_max, y_min, y_max], decode) =
        decode.split_first_chunk::<4>().map(|(a, b)| (*a, b))?;
    let num_components = decode.len() / 2;

    let mut reader = BitReader::new(data);

    let interpolate_coord = |n: u32, d_min: f32, d_max: f32| {
        interpolate(
            n as f32,
            0.0,
            2.0f32.powi(bp_cord.bits() as i32) - 1.0,
            d_min,
            d_max,
        )
    };

    let interpolate_comp = |n: u32, d_min: f32, d_max: f32| {
        interpolate(
            n as f32,
            0.0,
            2.0f32.powi(bp_comp.bits() as i32) - 1.0,
            d_min,
            d_max,
        )
    };

    let read_single = |reader: &mut BitReader, has_function: bool| -> Option<TriangleVertex> {
        let flag = reader.read(bpf)?;
        let x = interpolate_coord(reader.read(bp_cord)?, x_min, x_max);
        let y = interpolate_coord(reader.read(bp_cord)?, y_min, y_max);

        let mut colors = vec![];

        if has_function {
            // Just read the parametric value.
            colors.push(interpolate_comp(
                reader.read(bp_comp)?,
                decode[0],
                decode[1],
            ));
        } else {
            for (_, decode) in (0..num_components).zip(decode.chunks_exact(2)) {
                colors.push(interpolate_comp(
                    reader.read(bp_comp)?,
                    decode[0],
                    decode[1],
                ));
            }
        }

        reader.align();

        Some(TriangleVertex { flag, x, y, colors })
    };

    let mut a = None;
    let mut b = None;
    let mut c = None;

    loop {
        let Some(first) = read_single(&mut reader, function.is_some()) else {
            break;
        };

        if first.flag == 0 {
            let second = read_single(&mut reader, function.is_some())?;
            let third = read_single(&mut reader, function.is_some())?;

            a = Some(first.clone());
            b = Some(second.clone());
            c = Some(third.clone());
        } else if first.flag == 1 {
            a = Some(b.clone()?);
            b = Some(c.clone()?);
            c = Some(first);
        } else if first.flag == 2 {
            b = Some(c.clone()?);
            c = Some(first);
        }

        triangles.push(Triangle {
            p0: a.clone()?,
            p1: b.clone()?,
            p2: c.clone()?,
        })
    }

    Some(triangles)
}

fn read_lattice_triangles(
    data: &[u8],
    bp_cord: u8,
    bp_comp: u8,
    function: Option<&Function>,
    vertices_per_row: u32,
    decode: &[f32],
) -> Option<Vec<Triangle>> {
    let bp_cord = BitSize::from_u8(bp_cord)?;
    let bp_comp = BitSize::from_u8(bp_comp)?;

    let mut lattices = vec![];

    let ([x_min, x_max, y_min, y_max], decode) =
        decode.split_first_chunk::<4>().map(|(a, b)| (*a, b))?;
    let num_components = decode.len() / 2;

    let mut reader = BitReader::new(data);

    let interpolate_coord = |n: u32, d_min: f32, d_max: f32| {
        interpolate(
            n as f32,
            0.0,
            2.0f32.powi(bp_cord.bits() as i32) - 1.0,
            d_min,
            d_max,
        )
    };

    let interpolate_comp = |n: u32, d_min: f32, d_max: f32| {
        interpolate(
            n as f32,
            0.0,
            2.0f32.powi(bp_comp.bits() as i32) - 1.0,
            d_min,
            d_max,
        )
    };

    let read_single = |reader: &mut BitReader, has_function: bool| -> Option<TriangleVertex> {
        let x = interpolate_coord(reader.read(bp_cord)?, x_min, x_max);
        let y = interpolate_coord(reader.read(bp_cord)?, y_min, y_max);

        let mut colors = vec![];

        if has_function {
            // Just read the parametric value.
            colors.push(interpolate_comp(
                reader.read(bp_comp)?,
                decode[0],
                decode[1],
            ));
        } else {
            for (_, decode) in (0..num_components).zip(decode.chunks_exact(2)) {
                colors.push(interpolate_comp(
                    reader.read(bp_comp)?,
                    decode[0],
                    decode[1],
                ));
            }
        }

        reader.align();

        Some(TriangleVertex { flag: 0, x, y, colors })
    };

    'outer: loop {
        let mut single_row = vec![];
        
        for _ in 0..vertices_per_row {
            let Some(next) = read_single(&mut reader, function.is_some()) else {
                break 'outer;
            };

            single_row.push(next);
        }
        
        lattices.push(single_row);
    }
    
    let mut triangles = vec![];
    
    for i in 0..(lattices.len() - 1) {
        for j in 0..(vertices_per_row as usize - 1) {
            triangles.push(Triangle {
                p0: lattices[i][j].clone(),
                p1: lattices[i + 1][j].clone(),
                p2: lattices[i][j + 1].clone(),
            });

            triangles.push(Triangle {
                p0: lattices[i + 1][j + 1].clone(),
                p1: lattices[i + 1][j].clone(),
                p2: lattices[i][j + 1].clone(),
            });
        }
    }

    Some(triangles)
}
