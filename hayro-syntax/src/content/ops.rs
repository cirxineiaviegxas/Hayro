// Compatibility operators

use crate::content::{OPERANDS_THRESHOLD, Operation, OperatorTrait, Stack};
use crate::object::Object;
use crate::object::array::Array;
use crate::object::name::Name;
use crate::object::number::Number;
use crate::object::string;
use smallvec::SmallVec;

use crate::{op_all, op0, op1, op2, op3, op4, op6};
use log::warn;

include!("ops_generated.rs");

#[cfg(test)]
mod tests {
    use crate::content::ops::{
        ClosePath, FillPathNonZero, LineTo, MoveTo, NonStrokeColorDeviceRgb, SetGraphicsState,
        Transform, TypedOperation,
    };
    use crate::content::{TypedIter, UntypedIter};
    use crate::object::name::Name;
    use crate::object::number::Number;

    fn n(num: i32) -> Number {
        Number::from_i32(num)
    }

    fn f(num: f32) -> Number {
        Number::from_f32(num)
    }

    #[test]
    fn basic_ops_1() {
        let input = b"
1 0 0 -1 0 200 cm
/g0 gs
1 0 0 rg
";

        let expected = vec![
            TypedOperation::Transform(Transform(n(1), n(0), n(0), n(-1), n(0), n(200))),
            TypedOperation::SetGraphicsState(SetGraphicsState(Name::from_unescaped(b"g0"))),
            TypedOperation::NonStrokeColorDeviceRgb(NonStrokeColorDeviceRgb(n(1), n(0), n(0))),
        ];

        let elements = TypedIter::new(UntypedIter::new(input))
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(elements, expected,)
    }

    #[test]
    fn basic_ops_2() {
        let input = b"
20 20 m
180 20 l
180.1 180.1 l
20 180 l
h
f
";

        let expected = vec![
            TypedOperation::MoveTo(MoveTo(n(20), n(20))),
            TypedOperation::LineTo(LineTo(n(180), n(20))),
            TypedOperation::LineTo(LineTo(f(180.1), f(180.1))),
            TypedOperation::LineTo(LineTo(n(20), n(180))),
            TypedOperation::ClosePath(ClosePath),
            TypedOperation::FillPathNonZero(FillPathNonZero),
        ];

        let elements = TypedIter::new(UntypedIter::new(input))
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(elements, expected,)
    }
}
