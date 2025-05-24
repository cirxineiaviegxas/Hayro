mod type0;
mod type2;
mod type3;
mod type4;

use crate::function::type0::Type0;
use crate::function::type2::Type2;
use crate::function::type3::Type3;
use crate::function::type4::Type4;
use crate::object::Object;
use crate::object::dict::Dict;
use crate::object::dict::keys::{DOMAIN, FUNCTION_TYPE, RANGE};
use crate::object::stream::Stream;
use log::warn;
use smallvec::SmallVec;
use std::sync::Arc;

type Values = SmallVec<[f32; 4]>;
type TupleVec = SmallVec<[(f32, f32); 4]>;

#[derive(Debug)]
enum FunctionType {
    Type0(Type0),
    Type2(Type2),
    Type3(Type3),
    Type4(Type4),
}

#[derive(Debug, Clone)]
pub struct Function {
    function_type: Arc<FunctionType>,
}

impl Function {
    pub fn new(obj: &Object) -> Option<Function> {
        let (dict, stream) = dict_or_stream(obj)?;

        let function_type = match dict.get::<u8>(FUNCTION_TYPE)? {
            0 => FunctionType::Type0(Type0::new(&stream?)?),
            2 => FunctionType::Type2(Type2::new(&dict)?),
            3 => FunctionType::Type3(Type3::new(&dict)?),
            4 => FunctionType::Type4(Type4::new(&stream?)?),
            _ => return None,
        };

        Some(Self {
            function_type: Arc::new(function_type),
        })
    }

    pub fn eval(&self, input: Values) -> Option<Values> {
        match self.function_type.as_ref() {
            FunctionType::Type0(t0) => t0.eval(input),
            FunctionType::Type2(t2) => Some(t2.eval(*input.get(0)?)),
            FunctionType::Type3(t3) => t3.eval(*input.get(0)?),
            FunctionType::Type4(t4) => Some(t4.eval(input)?),
        }
    }
}

#[derive(Debug, Clone)]
struct Clamper {
    domain: TupleVec,
    range: Option<TupleVec>,
}

impl Clamper {
    pub fn new(dict: &Dict) -> Option<Self> {
        let domain = dict.get::<TupleVec>(DOMAIN)?;
        let range = dict.get::<TupleVec>(RANGE);

        Some(Self { domain, range })
    }

    pub(crate) fn clamp_input(&self, input: &mut [f32]) {
        if input.len() != self.domain.len() {
            warn!("the domain of the function didn't match the input arguments");
        }

        for ((min, max), val) in self.domain.iter().zip(input.iter_mut()) {
            *val = val.clamp(*min, *max);
        }
    }

    pub(crate) fn clamp_output(&self, output: &mut [f32]) {
        if let Some(range) = &self.range {
            if range.len() != output.len() {
                warn!("the range of the function didn't match the output arguments");
            }

            for ((min, max), val) in range.iter().zip(output.iter_mut()) {
                *val = val.clamp(*min, *max);
            }
        }
    }
}

pub fn interpolate(x: f32, x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> f32 {
    y_min + (x - x_min) * (y_max - y_min) / (x_max - x_min)
}

pub fn dict_or_stream<'a>(obj: &Object<'a>) -> Option<(Dict<'a>, Option<Stream<'a>>)> {
    if let Some(stream) = obj.clone().cast::<Stream>() {
        Some((stream.dict().clone(), Some(stream)))
    } else if let Some(dict) = obj.clone().cast::<Dict>() {
        Some((dict, None))
    } else {
        None
    }
}
