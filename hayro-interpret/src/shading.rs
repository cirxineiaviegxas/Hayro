/// PDF shadings.
use crate::color::{ColorComponents, ColorSpace};
use hayro_syntax::bit::{BitReader, BitSize};
use hayro_syntax::function::{Function, Values, interpolate};
use hayro_syntax::object::Object;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{
    BACKGROUND, BBOX, BITS_PER_COMPONENT, BITS_PER_COORDINATE, BITS_PER_FLAG, COLORSPACE, COORDS,
    DECODE, DOMAIN, EXTEND, FUNCTION, MATRIX, SHADING_TYPE, VERTICES_PER_ROW,
};
use hayro_syntax::object::rect::Rect;
use hayro_syntax::object::stream::Stream;
use kurbo::{Affine, Point};
use log::warn;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;
use crate::util::PointExt;

/// The function supplied to a shading.
#[derive(Debug, Clone)]
pub enum ShadingFunction {
    /// A single function, which should be used to evaluate all components of the shading.
    Single(Function),
    /// Multiple functions, one for each color component.
    Multiple(SmallVec<[Function; 4]>),
}

impl ShadingFunction {
    /// Evaluate the shading function.
    pub fn eval(&self, input: &Values) -> Option<Values> {
        match self {
            ShadingFunction::Single(s) => s.eval(input.clone()),
            ShadingFunction::Multiple(m) => {
                // 1-in, 1-out function for each color component.

                let mut out = smallvec![];

                for func in m {
                    out.push(*func.eval(input.clone())?.get(0)?);
                }

                Some(out)
            }
        }
    }
}

/// A type of shading
#[derive(Debug)]
pub enum ShadingType {
    /// A function-based shading.
    FunctionBased {
        /// The domain of the function.
        domain: [f32; 4],
        /// A transform to apply to the shading.
        matrix: Affine,
        /// The function that should be used to evaluate the shading.
        function: ShadingFunction,
    },
    /// A radial-axial shading.
    RadialAxial {
        /// The coordinates of the shading.
        ///
        /// For axial shadings, only the first 4 entries are relevant, representing the x/y coordinates
        /// of the first point and the coordinates for the second point.
        ///
        /// For radial shadings, the coordinates contain the x/y coordinates as well as the radius
        /// for both circles.
        coords: [f32; 6],
        /// The domain of the shading.
        domain: [f32; 2],
        /// The function forming the basis of the shading.
        function: ShadingFunction,
        /// The extends in the left/right direction of the shading.
        extend: [bool; 2],
        /// Whether the shading is axial or radial.
        axial: bool,
    },
    /// A triangle-mesh shading.
    TriangleMesh {
        /// The triangles making up the shading.
        triangles: Vec<Triangle>,
        /// An optional function used for calculating the sampled color values.
        function: Option<ShadingFunction>,
    },
    /// A coons-patch-mesh shading.
    CoonsPatchMesh {
        /// The patches that make up the shading.
        patches: Vec<CoonsPatch>,
        /// An optional function used for calculating the sampled color values.
        function: Option<ShadingFunction>,
    },
}

/// A PDF shading.
#[derive(Clone, Debug)]
pub struct Shading {
    /// The type of shading.
    pub shading_type: Arc<ShadingType>,
    /// The color space of the shading.
    pub color_space: ColorSpace,
    /// The bounding box of the shading.
    pub bbox: Option<Rect>,
    /// The background color of the shading.
    pub background: Option<SmallVec<[f32; 4]>>,
}

impl Shading {
    pub fn new(dict: &Dict, stream: Option<&Stream>) -> Option<Self> {
        let shading_num = dict.get::<u8>(SHADING_TYPE)?;

        let color_space = ColorSpace::new(dict.get(COLORSPACE)?);

        let shading_type = match shading_num {
            1 => {
                let domain = dict.get::<[f32; 4]>(DOMAIN).unwrap_or([0.0, 1.0, 0.0, 1.0]);
                let matrix = dict
                    .get::<[f64; 6]>(MATRIX)
                    .map(|f| Affine::new(f))
                    .unwrap_or_default();
                let function = read_function(dict, &color_space)?;

                ShadingType::FunctionBased {
                    domain,
                    matrix,
                    function,
                }
            }
            2 | 3 => {
                let domain = dict.get::<[f32; 2]>(DOMAIN).unwrap_or([0.0, 1.0]);
                let function = read_function(dict, &color_space)?;
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
                let stream_data = stream.decoded()?;
                let bp_coord = dict.get::<u8>(BITS_PER_COORDINATE)?;
                let bp_comp = dict.get::<u8>(BITS_PER_COMPONENT)?;
                let bpf = dict.get::<u8>(BITS_PER_FLAG)?;
                let function = read_function(dict, &color_space);
                let decode = dict.get::<Array>(DECODE)?.iter::<f32>().collect::<Vec<_>>();

                let triangles = read_free_form_triangles(
                    stream_data.as_ref(),
                    bpf,
                    bp_coord,
                    bp_comp,
                    function.is_some(),
                    &decode,
                )?;

                ShadingType::TriangleMesh {
                    triangles,
                    function,
                }
            }
            5 => {
                let stream = stream?;
                let stream_data = stream.decoded()?;
                let bp_coord = dict.get::<u8>(BITS_PER_COORDINATE)?;
                let bp_comp = dict.get::<u8>(BITS_PER_COMPONENT)?;
                let function = read_function(dict, &color_space);
                let decode = dict.get::<Array>(DECODE)?.iter::<f32>().collect::<Vec<_>>();
                let vertices_per_row = dict.get::<u32>(VERTICES_PER_ROW)?;

                let triangles = read_lattice_triangles(
                    stream_data.as_ref(),
                    bp_coord,
                    bp_comp,
                    function.is_some(),
                    vertices_per_row,
                    &decode,
                )?;

                ShadingType::TriangleMesh {
                    triangles,
                    function,
                }
            }
            6 => {
                let stream = stream?;
                let stream_data = stream.decoded()?;
                let bp_coord = dict.get::<u8>(BITS_PER_COORDINATE)?;
                let bp_comp = dict.get::<u8>(BITS_PER_COMPONENT)?;
                let bpf = dict.get::<u8>(BITS_PER_FLAG)?;
                let function = read_function(dict, &color_space);
                let decode = dict.get::<Array>(DECODE)?.iter::<f32>().collect::<Vec<_>>();

                let patches = read_coons_patch_mesh(
                    stream_data.as_ref(),
                    bpf,
                    bp_coord,
                    bp_comp,
                    function.is_some(),
                    &decode,
                )?;

                ShadingType::CoonsPatchMesh { patches, function }
            }
            _ => return None,
        };

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

/// A triangle made up of three vertices.
#[derive(Clone, Debug)]
pub struct Triangle {
    /// The first vertex.
    pub p0: TriangleVertex,
    /// The second vertex.
    pub p1: TriangleVertex,
    /// The third vertex.
    pub p2: TriangleVertex,
}

/// A triangle vertex.
#[derive(Clone, Debug)]
pub struct TriangleVertex {
    flag: u32,
    /// The position of the vertex.
    pub point: Point,
    /// The color component of the vertex.
    pub colors: ColorComponents,
}

/// A coons patch.
#[derive(Clone, Debug)]
pub struct CoonsPatch {
    /// The control points of the coons patch.
    pub control_points: [Point; 12],
    /// The colors at each corner of the coons patch.
    pub colors: [ColorComponents; 4],
}

fn read_free_form_triangles(
    data: &[u8],
    bpf: u8,
    bp_cord: u8,
    bp_comp: u8,
    has_function: bool,
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

    let read_single = |reader: &mut BitReader| -> Option<TriangleVertex> {
        let flag = reader.read(bpf)?;
        let x = interpolate_coord(reader.read(bp_cord)?, x_min, x_max);
        let y = interpolate_coord(reader.read(bp_cord)?, y_min, y_max);

        let mut colors = smallvec![];

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

        let point = Point::new(x as f64, y as f64);

        Some(TriangleVertex {
            flag,
            point,
            colors,
        })
    };

    let mut a = None;
    let mut b = None;
    let mut c = None;

    loop {
        let Some(first) = read_single(&mut reader) else {
            break;
        };

        if first.flag == 0 {
            let second = read_single(&mut reader)?;
            let third = read_single(&mut reader)?;

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
        
        let (p0, p1, p2) = (a.clone()?, b.clone()?, c.clone()?);
        
        if p0.point.nearly_same(p1.point) || p1.point.nearly_same(p2.point) {
            return None;
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
    has_function: bool,
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

    let read_single = |reader: &mut BitReader| -> Option<TriangleVertex> {
        let x = interpolate_coord(reader.read(bp_cord)?, x_min, x_max);
        let y = interpolate_coord(reader.read(bp_cord)?, y_min, y_max);

        let mut colors = smallvec![];

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

        let point = Point::new(x as f64, y as f64);

        Some(TriangleVertex {
            flag: 0,
            point,
            colors,
        })
    };

    'outer: loop {
        let mut single_row = vec![];

        for _ in 0..vertices_per_row {
            let Some(next) = read_single(&mut reader) else {
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

fn read_coons_patch_mesh(
    data: &[u8],
    bpf: u8,
    bp_coord: u8,
    bp_comp: u8,
    has_function: bool,
    decode: &[f32],
) -> Option<Vec<CoonsPatch>> {
    let bpf = BitSize::from_u8(bpf)?;
    let bp_coord = BitSize::from_u8(bp_coord)?;
    let bp_comp = BitSize::from_u8(bp_comp)?;

    let ([x_min, x_max, y_min, y_max], decode) =
        decode.split_first_chunk::<4>().map(|(a, b)| (*a, b))?;
    let num_components = decode.len() / 2;

    let mut reader = BitReader::new(data);

    let interpolate_coord = |n: u32, d_min: f32, d_max: f32| {
        interpolate(
            n as f32,
            0.0,
            2.0f32.powi(bp_coord.bits() as i32) - 1.0,
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

    // Helper to read a single color (or t value)
    let read_colors = |reader: &mut BitReader| -> Option<ColorComponents> {
        let mut colors = smallvec![];
        if has_function {
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
        Some(colors)
    };

    // State for implicit control points/colors
    let mut prev_patch: Option<CoonsPatch> = None;
    let mut patches = vec![];

    while let Some(flag) = reader.read(bpf) {
        let mut control_points = [Point::ZERO; 12];
        let mut colors = [smallvec![], smallvec![], smallvec![], smallvec![]];

        match flag {
            0 => {
                for i in 0..12 {
                    let x = interpolate_coord(reader.read(bp_coord)?, x_min, x_max);
                    let y = interpolate_coord(reader.read(bp_coord)?, y_min, y_max);
                    control_points[i] = Point::new(x as f64, y as f64);
                }

                for i in 0..4 {
                    colors[i] = read_colors(&mut reader)?;
                }

                prev_patch = Some(CoonsPatch {
                    control_points,
                    colors: colors.clone(),
                });
            }
            1 => {
                let prev = prev_patch.as_ref()?;

                control_points[0] = prev.control_points[3];
                control_points[1] = prev.control_points[4];
                control_points[2] = prev.control_points[5];
                control_points[3] = prev.control_points[6];
                colors[0] = prev.colors[1].clone();
                colors[1] = prev.colors[2].clone();
                for i in 4..12 {
                    let x = interpolate_coord(reader.read(bp_coord)?, x_min, x_max);
                    let y = interpolate_coord(reader.read(bp_coord)?, y_min, y_max);
                    control_points[i] = Point::new(x as f64, y as f64);
                }

                for i in 2..4 {
                    colors[i] = read_colors(&mut reader)?;
                }

                prev_patch = Some(CoonsPatch {
                    control_points,
                    colors: colors.clone(),
                });
            }
            2 => {
                let prev = prev_patch.as_ref()?;
                control_points[0] = prev.control_points[6];
                control_points[1] = prev.control_points[7];
                control_points[2] = prev.control_points[8];
                control_points[3] = prev.control_points[9];
                colors[0] = prev.colors[2].clone();
                colors[1] = prev.colors[3].clone();

                for i in 4..12 {
                    let x = interpolate_coord(reader.read(bp_coord)?, x_min, x_max);
                    let y = interpolate_coord(reader.read(bp_coord)?, y_min, y_max);
                    control_points[i] = Point::new(x as f64, y as f64);
                }

                for i in 2..4 {
                    colors[i] = read_colors(&mut reader)?;
                }

                prev_patch = Some(CoonsPatch {
                    control_points,
                    colors: colors.clone(),
                });
            }
            3 => {
                let prev = prev_patch.as_ref()?;
                control_points[0] = prev.control_points[9];
                control_points[1] = prev.control_points[10];
                control_points[2] = prev.control_points[11];
                control_points[3] = prev.control_points[0];
                colors[0] = prev.colors[3].clone();
                colors[1] = prev.colors[0].clone();

                for i in 4..12 {
                    let x = interpolate_coord(reader.read(bp_coord)?, x_min, x_max);
                    let y = interpolate_coord(reader.read(bp_coord)?, y_min, y_max);
                    control_points[i] = Point::new(x as f64, y as f64);
                }

                for i in 2..4 {
                    colors[i] = read_colors(&mut reader)?;
                }

                prev_patch = Some(CoonsPatch {
                    control_points,
                    colors: colors.clone(),
                });
            }
            _ => break,
        }

        patches.push(CoonsPatch {
            control_points,
            colors,
        });
    }
    Some(patches)
}

fn read_function(dict: &Dict, color_space: &ColorSpace) -> Option<ShadingFunction> {
    if let Some(arr) = dict.get::<Array>(FUNCTION) {
        let arr: Option<SmallVec<_>> = arr.iter::<Object>().map(|o| Function::new(&o)).collect();
        let arr = arr?;

        if arr.len() != color_space.num_components() as usize {
            warn!("function array of shading has wrong size");

            return None;
        }

        Some(ShadingFunction::Multiple(arr))
    } else if let Some(obj) = dict.get::<Object>(FUNCTION) {
        Some(ShadingFunction::Single(Function::new(&obj)?))
    } else {
        None
    }
}
