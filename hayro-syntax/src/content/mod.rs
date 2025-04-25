pub mod ops;

use crate::content::TypedOperation::Fallback;
use crate::content::ops::TypedOperation;
use crate::file::xref::XRef;
use crate::object::array::Array;
use crate::object::name::{Name, escape_name_like, skip_name_like};
use crate::object::number::Number;
use crate::object::{Object, ObjectLike, string};
use crate::reader::{Readable, Reader, Skippable};
use log::warn;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};

// 6 operands are used for example for ctm or cubic curves,
// but anything above should be pretty rare (for example for
// DeviceN color spaces)
const OPERANDS_THRESHOLD: usize = 6;

type OpVec<'a> = SmallVec<[Object<'a>; OPERANDS_THRESHOLD]>;

impl Debug for Operator<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::str::from_utf8(&self.get()).unwrap())
    }
}

pub struct Operator<'a> {
    data: &'a [u8],
    has_escape: bool,
}

impl<'a> Operator<'a> {
    pub fn get(&self) -> Cow<'a, [u8]> {
        escape_name_like(self.data, self.has_escape)
    }
}

impl Skippable for Operator<'_> {
    fn skip<const PLAIN: bool>(r: &mut Reader<'_>) -> Option<()> {
        skip_name_like(r, false).map(|_| ())
    }
}

impl<'a> Readable<'a> for Operator<'a> {
    fn read<const PLAIN: bool>(r: &mut Reader<'a>, _: &XRef<'a>) -> Option<Self> {
        let (data, has_escape) = {
            let start = r.offset();
            let has_escape = skip_name_like(r, false)?;
            let end = r.offset();
            let data = r.range(start..end).unwrap();

            if data.is_empty() {
                return None;
            }

            (data, has_escape)
        };

        Some(Operator { data, has_escape })
    }
}

pub struct UntypedIter<'a> {
    reader: Reader<'a>,
    stack: Stack<'a>,
}

impl<'a> UntypedIter<'a> {
    pub fn new(data: &'a [u8]) -> UntypedIter<'a> {
        Self {
            reader: Reader::new(data),
            stack: Stack::new(),
        }
    }

    pub fn empty() -> UntypedIter<'a> {
        Self {
            reader: Reader::new(&[]),
            stack: Stack::new(),
        }
    }
}

impl<'a> Iterator for UntypedIter<'a> {
    type Item = Operation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.clear();

        self.reader.skip_white_spaces_and_comments();

        while !self.reader.at_end() {
            // I believe booleans/null never appear as an operator?
            if matches!(
                self.reader.peek_byte()?,
                b'/' | b'.' | b'+' | b'-' | b'0'..=b'9' | b'[' | b'<' | b'('
            ) {
                self.stack.push(self.reader.read_plain::<Object>()?);
            } else {
                let operator = match self.reader.read_plain::<Operator>() {
                    Some(o) => o,
                    None => {
                        warn!("failed to read operator");

                        self.reader.jump_to_end();
                        return None;
                    }
                };

                // Hack for now to skip inline images, which form an exception.
                if operator.get().as_ref() == b"BI" {
                    while let Some(bytes) = self.reader.read_bytes(2) {
                        if bytes == b"EI" {
                            self.reader.skip_white_spaces();

                            break;
                        }
                    }
                }

                return Some(Operation {
                    operands: self.stack.clone(),
                    operator,
                });
            }

            self.reader.skip_white_spaces_and_comments();
        }

        None
    }
}

pub struct TypedIter<'a> {
    untyped: UntypedIter<'a>,
}

impl<'a> TypedIter<'a> {
    pub fn new(untyped: UntypedIter<'a>) -> TypedIter<'a> {
        Self { untyped }
    }
}

impl<'a> Iterator for TypedIter<'a> {
    type Item = TypedOperation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.untyped
            .next()
            .and_then(|op| TypedOperation::dispatch(&op))
    }
}

pub struct Operation<'a> {
    pub operands: Stack<'a>,
    pub operator: Operator<'a>,
}

impl<'a> Operation<'a> {
    pub fn operands(self) -> OperandIterator<'a> {
        OperandIterator::new(self.operands)
    }
}

#[derive(Debug, Clone)]
pub struct Stack<'a>(SmallVec<[Object<'a>; OPERANDS_THRESHOLD]>);

impl<'a> Stack<'a> {
    pub fn new() -> Self {
        Self(SmallVec::new())
    }

    fn push(&mut self, operand: Object<'a>) {
        self.0.push(operand);
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn get<T>(&self, index: usize) -> Option<T>
    where
        T: ObjectLike<'a>,
    {
        self.0.get(index).and_then(|e| e.clone().cast::<T>().ok())
    }

    fn get_all<T>(&self) -> Option<SmallVec<[T; OPERANDS_THRESHOLD]>>
    where
        T: ObjectLike<'a>,
    {
        let mut operands = SmallVec::new();

        for op in &self.0 {
            let converted = op.clone().cast::<T>().ok()?;
            operands.push(converted);
        }

        Some(operands)
    }
}

pub struct OperandIterator<'a> {
    stack: Stack<'a>,
    cur_index: usize,
}

impl<'a> OperandIterator<'a> {
    fn new(stack: Stack<'a>) -> Self {
        Self {
            stack,
            cur_index: 0,
        }
    }
}

impl<'a> Iterator for OperandIterator<'a> {
    type Item = Object<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.stack.get::<Object<'a>>(self.cur_index) {
            self.cur_index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub(crate) trait OperatorTrait<'a>
where
    Self: Sized + Into<TypedOperation<'a>> + TryFrom<TypedOperation<'a>>,
{
    const OPERATOR: &'static str;

    fn from_stack(stack: &Stack<'a>) -> Option<Self>;
}

#[macro_export]
macro_rules! op_impl {
    ($t:ident $(<$l:lifetime>),*, $e:expr, $n:expr, $body:expr) => {
        impl<'a> OperatorTrait<'a> for $t$(<$l>),* {
            const OPERATOR: &'static str = $e;

            fn from_stack(stack: &Stack<'a>) -> Option<Self> {
                if $n != u8::MAX as usize {
                    if stack.len() != $n {
                        warn!("wrong stack length {} for operator {}, expected {}", stack.len(), Self::OPERATOR, $n);

                        return None;
                    }
                }

                $body(stack).or_else(|| {
                    warn!("failed to convert operands for operator {}", Self::OPERATOR);

                    None
                })
            }
        }

        impl<'a> From<$t$(<$l>),*> for TypedOperation<'a> {
            fn from(value: $t$(<$l>),*) -> Self {
                TypedOperation::$t(value)
            }
        }

        impl<'a> TryFrom<TypedOperation<'a>> for $t$(<$l>),* {
            type Error = ();

            fn try_from(value: TypedOperation<'a>) -> std::result::Result<Self, Self::Error> {
                match value {
                    TypedOperation::$t(e) => Ok(e),
                    _ => Err(())
                }
            }
        }
    };
}

#[macro_export]
macro_rules! op0 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        crate::op_impl!($t$(<$l>),*, $e, 0, |_| Some(Self));
    }
}

#[macro_export]
macro_rules! op1 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        crate::op_impl!($t$(<$l>),*, $e, 1, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?)));
    }
}

#[macro_export]
macro_rules! op_all {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        crate::op_impl!($t$(<$l>),*, $e, u8::MAX as usize, |stack: &Stack<'a>|
        Some(Self(stack.get_all()?)));
    }
}

#[macro_export]
macro_rules! op2 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        crate::op_impl!($t$(<$l>),*, $e, 2, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?)));
    }
}

#[macro_export]
macro_rules! op3 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        crate::op_impl!($t$(<$l>),*, $e, 3, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?,
        stack.get(2)?)));
    }
}

#[macro_export]
macro_rules! op4 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        crate::op_impl!($t$(<$l>),*, $e, 4, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?,
        stack.get(2)?, stack.get(3)?)));
    }
}

#[macro_export]
macro_rules! op6 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        crate::op_impl!($t$(<$l>),*, $e, 6, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?,
        stack.get(2)?, stack.get(3)?,
        stack.get(4)?, stack.get(5)?)));
    }
}
