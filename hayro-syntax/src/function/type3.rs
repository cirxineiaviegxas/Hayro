use crate::function::{DomainRange, Function, Values, interpolate, read_domain_range};
use crate::object::Object;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{BOUNDS, ENCODE, FUNCTIONS, RANGE};
use smallvec::smallvec;

/// Type 2 exponential interpolation function.
#[derive(Debug)]
pub(crate) struct Type3 {
    functions: Vec<Function>,
    bounds: Vec<f32>,
    encode: DomainRange,
    domain: (f32, f32),
}

impl Type3 {
    pub(crate) fn new(dict: &Dict, domain: &DomainRange) -> Option<Self> {
        let functions = dict
            .get::<Array>(FUNCTIONS)
            .and_then(|d| d.iter::<Object>().map(|o| Function::new(&o)).collect())?;
        let domain = *domain.get(0)?;
        let mut bounds = vec![domain.0 - 0.0001];
        dict.get::<Array>(BOUNDS)
            .map(|a| bounds.extend(a.iter::<f32>()));
        // Add a small delta so that the interval is considered to be closed on the right.
        bounds.push(domain.1 + 0.0001);

        let encode = dict
            .get::<Array>(ENCODE)
            .and_then(|a| read_domain_range(&a))?;

        Some(Self {
            functions,
            bounds,
            encode,
            domain,
        })
    }

    pub(crate) fn eval(&self, input: f32) -> Option<Values> {
        let index = find_interval(&self.bounds, input)?;

        let bounds_i = *self.bounds.get(index + 1)?;
        let bounds_i_minus_1 = *self.bounds.get(index)?;

        // - 1 because we inserted a dummy bound in the constructor.
        let encoding = self.encode.get(index)?;
        let function = self.functions.get(index)?;
        let encoded = interpolate(input, bounds_i_minus_1, bounds_i, encoding.0, encoding.1);

        function.eval(smallvec![encoded])
    }
}

fn find_interval(bounds: &[f32], x: f32) -> Option<usize> {
    if x < *bounds.get(0)? || x >= *bounds.last()? {
        return None;
    }

    match bounds.binary_search_by(|val| {
        if *val <= x {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }) {
        Ok(i) => Some(i - 1),
        Err(i) => Some(i - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::type2::Type2;
    use crate::function::{Clamper, FunctionType};
    use crate::reader::Readable;
    use std::sync::Arc;

    #[test]
    fn simple() {
        let data = b"<<
  /FunctionType 3
  /Domain [-7 7]
  /Functions [
    << /FunctionType 2
       /Domain [0 1]
       /C0 [0.5 0.5 0.5]
       /C1 [0.5 0.5 0.5]
       /N 1
    >>
    << /FunctionType 2
       /Domain [0 1]
       /C0 [0.7 0.7 0.7]
       /C1 [0.7 0.7 0.7]
       /N 1
    >>
  ]
  /Bounds [0]
  /Encode [0 1 0 1]
>>";

        let dict = Object::from_bytes(data).unwrap();
        let function = Function::new(&dict).unwrap();

        assert_eq!(
            function.eval(smallvec![-7.0]).unwrap().as_slice(),
            &[0.5, 0.5, 0.5]
        );
        assert_eq!(
            function.eval(smallvec![-3.0]).unwrap().as_slice(),
            &[0.5, 0.5, 0.5]
        );
        assert_eq!(
            function.eval(smallvec![-0.5]).unwrap().as_slice(),
            &[0.5, 0.5, 0.5]
        );
        assert_eq!(
            function.eval(smallvec![0.0]).unwrap().as_slice(),
            &[0.7, 0.7, 0.7]
        );
        assert_eq!(
            function.eval(smallvec![7.0]).unwrap().as_slice(),
            &[0.7, 0.7, 0.7]
        );
    }
}
