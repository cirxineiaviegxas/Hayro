mod type2;
mod type4;
mod type0;

use crate::function::type2::Type2;
use crate::function::type4::Type4;
use crate::object::Object;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{DOMAIN, FUNCTION_TYPE, RANGE};
use crate::object::number::Number;
use crate::object::stream::Stream;
use log::{error, warn};
use smallvec::{smallvec, SmallVec};
use crate::function::type0::Type0;

type Values = SmallVec<[f32; 4]>;
type DomainRange = SmallVec<[(f32, f32); 4]>;

#[derive(Debug)]
enum FunctionType {
    Type0(Type0),
    Type2(Type2),
    Type4(Type4),
}

#[derive(Debug)]
pub struct Function {
    function_type: FunctionType,
    domain: Clamper,
    range: Option<Clamper>,
}

impl Function {
    pub fn new(obj: &Object) -> Option<Function> {
        let (dict, stream) = if let Some(stream) = obj.clone().cast::<Stream>().ok() {
            (stream.dict().clone(), Some(stream))
        } else if let Some(dict) = obj.clone().cast::<Dict>().ok() {
            (dict, None)
        } else {
            return None;
        };

        let domain = dict.get::<Array>(DOMAIN).and_then(|a| read_domain_range(&a))?;
        let range = dict.get::<Array>(RANGE).and_then(|a| read_domain_range(&a));

        let function_type = match dict.get::<u8>(FUNCTION_TYPE)? {
            0 => FunctionType::Type0(Type0::new(&stream?, &domain, range.as_ref()?)?),
            2 => FunctionType::Type2(Type2::new(&dict)?),
            4 => FunctionType::Type4(Type4::new(&stream?)?),
            _ => return None,
        };

        Some(Self {
            domain: Clamper(domain),
            range: range.map(|a| Clamper(a)),
            function_type,
        })
    }

    pub fn eval(&self, mut input: Values) -> Option<Values> {
        self.clamp_domain(&mut input)?;

        match &self.function_type {
            FunctionType::Type0(t0) => t0.eval(input),
            FunctionType::Type2(t2) => Some(t2.eval(*input.get(0)?)),
            FunctionType::Type4(t4) => Some(t4.eval(input)?),
        }
        .map(|mut v| {
            let _ = self.clamp_range(&mut v);
            v
        })
    }

    #[must_use]
    fn clamp_domain(&self, input: &mut Values) -> Option<()> {
        if input.len() != self.domain.dimension() {
            error!("mismatch while clamping domain of postscript function");

            return None;
        }

        for (idx, val) in input.iter_mut().enumerate() {
            self.clamp_domain_single(val, idx);
        }

        Some(())
    }

    fn clamp_domain_single(&self, val: &mut f32, idx: usize) {
        *val = self.domain.clamp(*val, idx);
    }

    #[must_use]
    fn clamp_range(&self, input: &mut Values) -> Option<()> {
        if let Some(range) = &self.range {
            if input.len() != range.dimension() {
                error!("mismatch while clamping range of postscript function");

                return None;
            }

            for (idx, val) in input.iter_mut().enumerate() {
                *val = range.clamp(*val, idx);
            }
        }

        Some(())
    }
}

fn read_domain_range(array: &Array) -> Option<DomainRange> {
    let mut iter = array.iter::<f32>();
    let mut vals = smallvec![];
    
    while let Some(first) = iter.next() {
        let second = iter.next()?;
        
        vals.push((first, second));
    }
    
    Some(vals)
}

impl From<Array<'_>> for Values {
    fn from(value: Array) -> Self {
        value
            .iter::<Number>()
            .map(|n| n.as_f32())
            .collect::<Values>()
    }
}

#[derive(Debug)]
struct Clamper(DomainRange);

impl Clamper {
    fn clamp(&self, val: f32, idx: usize) -> f32 {
        if idx >= self.0.len() {
            warn!("the domain/range of the function was exceeded");
        }
        
        let (min, max) = self.0.get(idx).copied().unwrap_or((0.0, 0.0));

        val.clamp(min, max)
    }

    fn dimension(&self) -> usize {
        self.0.len()
    }
}

pub fn interpolate(x: f32, x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> f32 {
    y_min + (x - x_min) * (y_max - y_min) / (x_max - x_min)
}