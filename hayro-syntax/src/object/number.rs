use crate::file::xref::XRef;
use crate::object;
use crate::object::{Object, ObjectLike};
use crate::reader::{Readable, Reader, Skippable};
use std::fmt::Debug;

/// A PDF number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Number(pub(crate) InternalNumber);

impl Number {
    /// Returns the number as an f64.
    pub fn as_f64(&self) -> f64 {
        match self.0 {
            InternalNumber::Real(r) => r as f64,
            InternalNumber::Integer(i) => i as f64,
        }
    }

    /// Returns the number as a f32, if possible.
    pub fn as_f32(&self) -> Option<f32> {
        match self.0 {
            InternalNumber::Real(r) => Some(r),
            InternalNumber::Integer(i) => {
                let converted = i as f32;

                // Double check whether conversion didn't overflow.
                if converted as i32 != i {
                    None
                } else {
                    Some(converted)
                }
            }
        }
    }

    /// Returns the number as an i32, if possible.
    pub fn as_i32(&self) -> Option<i32> {
        match self.0 {
            InternalNumber::Real(r) => {
                if r.trunc() == r {
                    Some(r as i32)
                } else {
                    None
                }
            }
            InternalNumber::Integer(i) => Some(i),
        }
    }

    pub(crate) fn from_f32(num: f32) -> Self {
        Self(InternalNumber::Real(num))
    }

    pub(crate) fn from_i32(num: i32) -> Self {
        Self(InternalNumber::Integer(num))
    }
}

impl Skippable for Number {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.forward_if(|b| b == b'+' || b == b'-');

        match r.peek_byte()? {
            b'.' => {
                r.read_byte()?;
                r.forward_while_1(is_digit)?;
            }
            (b'0'..=b'9') => {
                r.forward_while_1(is_digit)?;
                if let Some(()) = r.forward_tag(b".") {
                    r.forward_while(is_digit);
                }
            }
            _ => return None,
        }

        Some(())
    }
}

impl Readable<'_> for Number {
    fn read<const PLAIN: bool>(r: &mut Reader<'_>, _: &XRef<'_>) -> Option<Self> {
        // Yes, this is not how you parse numbers correctly, especially floating point
        // numbers! The result might not be 100% accurate in many cases. The problem we are facing
        // is that parsing numbers is a huge bottleneck (after all 90% of content streams is usually
        // just numbers), so handling them efficiently is pretty important if we care about having
        // the highest speed.
        //
        // There are essentially three choices:
        // 1) First determine the boundary of the number, then convert it to string and finally use
        // the Rust `parse` implementation to parse the number as a float/int. However,
        // as mentioned, this is relatively slow.
        // 2) Use the `lexical` crate, which allows parsing (partial) numbers from a byte slice.
        // However, this crate has a lot of unsafe, so I'm a bit hesitant to use it, although I'm
        // sure it works just fine.
        // 3) Write our own "sloppy" implementation of parsing numbers, which is very fast and
        // safe, but unfortunately might result in a loss of precision for floating point numbers.
        //
        // I chose 3), because parsing PDF numbers does not strike me as a use case where numbers
        // have to be 100% accurate, so I think this trade-off should be worth it.
        // However, if at some point this bites us (hopefully not!), we can just revert
        // to option 1 or 2.

        let mut int_part: i64 = 0;
        let mut frac_part: f64 = 0.0;
        let mut frac_mul_factor: u64 = 1;

        let negative = match r.peek_byte()? {
            b'+' => {
                r.read_byte()?;
                false
            }
            b'-' => {
                r.read_byte()?;
                true
            }
            _ => false,
        };

        let mut parse_frac = |r: &mut Reader<'_>| -> Option<()> {
            while let Some(d) = r.peek_byte() {
                match d {
                    (b'0'..=b'9') => {
                        frac_part *= 10.0;
                        frac_part += (d - b'0') as f64;
                        frac_mul_factor *= 10;

                        r.read_byte()?;
                    }
                    _ => return Some(()),
                }
            }

            Some(())
        };

        match r.peek_byte()? {
            b'.' => {
                r.read_byte()?;
                parse_frac(r)?;
            }
            (b'0'..=b'9') => {
                while let Some(d) = r.peek_byte() {
                    match d {
                        (b'0'..=b'9') => {
                            int_part = int_part.checked_mul(10)?;
                            int_part = int_part.checked_add((d - b'0') as i64)?;
                            r.read_byte()?;
                        }
                        _ => break,
                    }
                }

                if let Some(()) = r.forward_tag(b".") {
                    parse_frac(r)?;
                }
            }
            _ => return None,
        }

        if frac_part == 0.0 {
            if negative {
                int_part = -int_part;
            }

            Some(Number(InternalNumber::Integer(int_part.try_into().ok()?)))
        } else {
            let mut res = (int_part as f64 + frac_part / frac_mul_factor as f64) as f32;

            if negative {
                res = -res;
            }

            Some(Number(InternalNumber::Real(res)))
        }
    }
}

object!(Number, Number);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum InternalNumber {
    Real(f32),
    Integer(i32),
}

macro_rules! int_num {
    ($i:ident) => {
        impl Skippable for $i {
            fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
                r.forward_if(|b| b == b'+' || b == b'-');
                r.forward_while_1(is_digit)?;

                // We have a float instead of an integer.
                if r.peek_byte() == Some(b'.') {
                    return None;
                }

                Some(())
            }
        }

        impl<'a> Readable<'a> for $i {
            fn read<const PLAIN: bool>(r: &mut Reader<'a>, xref: &XRef<'a>) -> Option<$i> {
                r.read::<PLAIN, Number>(xref)
                    .and_then(|n| n.as_i32())
                    .and_then(|n| n.try_into().ok())
            }
        }

        impl TryFrom<Object<'_>> for $i {
            type Error = ();

            fn try_from(value: Object<'_>) -> std::result::Result<Self, Self::Error> {
                match value {
                    Object::Number(n) => n.as_i32().and_then(|f| f.try_into().ok()).ok_or(()),
                    _ => Err(()),
                }
            }
        }

        impl<'a> ObjectLike<'a> for $i {
            const STATIC_NAME: &'static str = stringify!($i);
        }
    };
}

int_num!(i32);
int_num!(u32);
int_num!(usize);
int_num!(u8);

impl Skippable for f32 {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        r.skip::<PLAIN, Number>().map(|_| {})
    }
}

impl Readable<'_> for f32 {
    fn read<const PLAIN: bool>(r: &mut Reader, _: &XRef<'_>) -> Option<Self> {
        r.read_plain::<Number>().and_then(|n| n.as_f32())
    }
}

impl TryFrom<Object<'_>> for f32 {
    type Error = ();

    fn try_from(value: Object<'_>) -> Result<Self, Self::Error> {
        match value {
            Object::Number(n) => n.as_f32().ok_or(()),
            _ => Err(()),
        }
    }
}

pub(crate) fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}

#[cfg(test)]
mod tests {
    use crate::object::number::Number;
    use crate::reader::Reader;

    #[test]
    fn int_1() {
        assert_eq!(Reader::new("0".as_bytes()).read_plain::<i32>().unwrap(), 0);
    }

    #[test]
    fn int_3() {
        assert_eq!(
            Reader::new("+32".as_bytes()).read_plain::<i32>().unwrap(),
            32
        );
    }

    #[test]
    fn int_4() {
        assert_eq!(
            Reader::new("-32".as_bytes()).read_plain::<i32>().unwrap(),
            -32
        );
    }

    #[test]
    fn int_5() {
        assert_eq!(
            Reader::new("-32.01".as_bytes())
                .read_plain::<i32>()
                .is_none(),
            true
        );
    }

    #[test]
    fn int_6() {
        assert_eq!(
            Reader::new("98349".as_bytes()).read_plain::<i32>().unwrap(),
            98349
        );
    }

    #[test]
    fn int_7() {
        assert_eq!(
            Reader::new("003245".as_bytes())
                .read_plain::<i32>()
                .unwrap(),
            3245
        );
    }

    #[test]
    fn int_trailing() {
        assert_eq!(
            Reader::new("0abc".as_bytes()).read_plain::<i32>().unwrap(),
            0
        );
    }

    #[test]
    fn real_1() {
        assert_eq!(
            Reader::new("3".as_bytes()).read_plain::<f32>().unwrap(),
            3.0
        );
    }

    #[test]
    fn real_3() {
        assert_eq!(
            Reader::new("+32".as_bytes()).read_plain::<f32>().unwrap(),
            32.0
        );
    }

    #[test]
    fn real_4() {
        assert_eq!(
            Reader::new("-32".as_bytes()).read_plain::<f32>().unwrap(),
            -32.0
        );
    }

    #[test]
    fn real_5() {
        assert_eq!(
            Reader::new("-32.01".as_bytes())
                .read_plain::<f32>()
                .unwrap(),
            -32.01
        );
    }

    #[test]
    fn real_6() {
        assert_eq!(
            Reader::new("-.345".as_bytes()).read_plain::<f32>().unwrap(),
            -0.345
        );
    }

    #[test]
    fn real_7() {
        assert_eq!(
            Reader::new("-.00143".as_bytes())
                .read_plain::<f32>()
                .unwrap(),
            -0.00143
        );
    }

    #[test]
    fn real_8() {
        assert_eq!(
            Reader::new("-12.0013".as_bytes())
                .read_plain::<f32>()
                .unwrap(),
            -12.0013
        );
    }

    #[test]
    fn real_9() {
        assert_eq!(
            Reader::new("98349.432534".as_bytes())
                .read_plain::<f32>()
                .unwrap(),
            98349.432534
        );
    }

    #[test]
    fn real_10() {
        assert_eq!(
            Reader::new("-34534656.34".as_bytes())
                .read_plain::<f32>()
                .unwrap(),
            -34534656.34
        );
    }

    #[test]
    fn real_trailing() {
        assert_eq!(
            Reader::new("0abc".as_bytes()).read_plain::<f32>().unwrap(),
            0.0
        );
    }

    #[test]
    fn real_failing() {
        assert_eq!(
            Reader::new("+abc".as_bytes()).read_plain::<f32>().is_none(),
            true
        );
    }

    #[test]
    fn number_1() {
        assert_eq!(
            Reader::new("+32".as_bytes())
                .read_plain::<Number>()
                .unwrap()
                .as_f64() as f32,
            32.0
        );
    }

    #[test]
    fn number_2() {
        assert_eq!(
            Reader::new("-32.01".as_bytes())
                .read_plain::<Number>()
                .unwrap()
                .as_f64() as f32,
            -32.01
        );
    }

    #[test]
    fn number_3() {
        assert_eq!(
            Reader::new("-.345".as_bytes())
                .read_plain::<Number>()
                .unwrap()
                .as_f64() as f32,
            -0.345
        );
    }
}
