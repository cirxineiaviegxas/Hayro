use crate::function::{CommonProperties, Values};
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{C0, C1, N};
use crate::object::number::Number;
use itertools::izip;
use smallvec::smallvec;

/// Type 2 exponential interpolation function.
struct Type2 {
    common: CommonProperties,
    c0: Values,
    c1: Values,
    n: f32,
}

impl Type2 {
    fn eval(&self, mut input: f32) -> Values {
        self.common.clamp_domain_single(&mut input, 0);

        let mut out = smallvec![0.0; self.c0.len()];

        for (c0, c1, out) in izip!(&self.c0, &self.c1, &mut out) {
            *out = *c0 + input.powf(self.n) * (*c1 - *c0);
        }

        let _ = self.common.clamp_range(&mut out);

        out
    }
}

impl TryFrom<Dict<'_>> for Type2 {
    type Error = ();

    fn try_from(value: Dict<'_>) -> Result<Self, Self::Error> {
        let common = CommonProperties::try_from(value.clone())?;
        let c0 = value.get::<Array>(C0).ok_or(())?.into();
        let c1 = value.get::<Array>(C1).ok_or(())?.into();
        let n = value.get::<Number>(N).ok_or(())?.as_f32();

        Ok(Self { common, c0, c1, n })
    }
}

#[cfg(test)]
mod tests {
    use crate::function::type2::Type2;
    use crate::object::dict::Dict;
    use crate::reader::Readable;

    #[test]
    fn simple() {
        let d: Type2 = Dict::from_bytes(
            b"<<
              /FunctionType 2
              /Domain [ 0  1 ]
              /C0 [ 0 20  ]
              /C1 [ 30 -50 ]
              /N 1
            >>",
        )
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(d.eval(0.0).as_ref(), &[0.0, 20.0]);

        assert_eq!(d.eval(0.5).as_ref(), &[15.0, -15.0]);

        assert_eq!(d.eval(1.0).as_ref(), &[30.0, -50.0]);
    }

    #[test]
    fn with_exponent() {
        let d: Type2 = Dict::from_bytes(
            b"<<
              /FunctionType 2
              /Domain [ 0  1 ]
              /C0 [ 0  ]
              /C1 [ 30 ]
              /N 2
            >>",
        )
        .unwrap()
        .try_into()
        .unwrap();
        assert_eq!(d.eval(0.5).as_ref(), &[7.5]);
    }

    #[test]
    fn clamp_domain() {
        let d: Type2 = Dict::from_bytes(
            b"<<
              /FunctionType 2
              /Domain [ 0.2  0.8 ]
              /C0 [ 0  ]
              /C1 [ 30 ]
              /N 2
            >>",
        )
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(d.eval(0.0).as_ref(), d.eval(0.2).as_ref(),);

        assert_eq!(d.eval(-10.0).as_ref(), d.eval(0.2).as_ref(),);

        assert_eq!(d.eval(0.8).as_ref(), d.eval(0.8).as_ref(),);

        assert_eq!(d.eval(1.2).as_ref(), d.eval(1.0).as_ref(),);
    }

    #[test]
    fn clamp_range() {
        let d: Type2 = Dict::from_bytes(
            b"<<
              /FunctionType 2
              /Domain [ 0.0  1.0 ]
              /Range [10.0 20.0]
              /C0 [ 0  ]
              /C1 [ 30 ]
              /N 1
            >>",
        )
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(d.eval(0.0).as_ref(), &[10.0]);

        assert_eq!(d.eval(0.5).as_ref(), &[15.0]);

        assert_eq!(d.eval(1.0).as_ref(), &[20.0]);
    }
}
