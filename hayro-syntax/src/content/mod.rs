use crate::content::TypedOperation::Fallback;
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

trait OperatorTrait<'a>
where
    Self: Sized + Into<TypedOperation<'a>> + TryFrom<TypedOperation<'a>>,
{
    const OPERATOR: &'static str;

    fn from_stack(stack: &Stack<'a>) -> Option<Self>;
}

macro_rules! op_impl {
    ($t:ident $(<$l:lifetime>),*, $e:expr, $n:expr, $body:expr) => {
        impl<'a> OperatorTrait<'a> for $t$(<$l>),* {
            const OPERATOR: &'static str = $e;

            fn from_stack(stack: &Stack<'a>) -> Option<Self> {
                if stack.len() != $n {
                    warn!("wrong stack length {} for operator {}, expected {}", stack.len(), Self::OPERATOR, $n);

                    return None;
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

macro_rules! op0 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        op_impl!($t$(<$l>),*, $e, 0, |_| Some(Self));
    }
}

macro_rules! op1 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        op_impl!($t$(<$l>),*, $e, 1, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?)));
    }
}

macro_rules! op2 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        op_impl!($t$(<$l>),*, $e, 2, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?)));
    }
}

macro_rules! op3 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        op_impl!($t$(<$l>),*, $e, 3, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?,
        stack.get(2)?)));
    }
}

macro_rules! op4 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        op_impl!($t$(<$l>),*, $e, 4, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?,
        stack.get(2)?, stack.get(3)?)));
    }
}

macro_rules! op6 {
    ($t:ident $(<$l:lifetime>),*, $e:expr) => {
        op_impl!($t$(<$l>),*, $e, 6, |stack: &Stack<'a>|
        Some(Self(stack.get(0)?, stack.get(1)?,
        stack.get(2)?, stack.get(3)?,
        stack.get(4)?, stack.get(5)?)));
    }
}

// Compatibility operators

#[derive(Debug)]
pub struct BeginCompatibility;
op0!(BeginCompatibility, "BX");

#[derive(Debug)]
pub struct EndCompatibility;
op0!(EndCompatibility, "EX");

// Graphics state operators

#[derive(Debug)]
pub struct SaveState;
op0!(SaveState, "q");

#[derive(Debug)]
pub struct RestoreState;
op0!(RestoreState, "Q");

#[derive(Debug)]
pub struct Transform(
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
);
op6!(Transform, "cm");

#[derive(Debug)]
pub struct LineWidth(pub Number);
op1!(LineWidth, "w");

#[derive(Debug)]
pub struct LineCap(pub Number);
op1!(LineCap, "J");

#[derive(Debug)]
pub struct LineJoin(pub Number);
op1!(LineJoin, "j");

#[derive(Debug)]
pub struct MiterLimit(pub Number);
op1!(MiterLimit, "M");

#[derive(Debug)]
pub struct DashPattern<'a>(pub Array<'a>, pub Number);
op2!(DashPattern<'a>, "d");

#[derive(Debug)]
pub struct RenderingIntent<'a>(pub Name<'a>);
op1!(RenderingIntent<'a>, "d");

#[derive(Debug)]
pub struct FlatnessTolerance(pub Number);
op1!(FlatnessTolerance, "i");

#[derive(Debug)]
pub struct SetGraphicsState<'a>(pub Name<'a>);
op1!(SetGraphicsState<'a>, "gs");

// Path-construction operators

#[derive(Debug)]
pub struct MoveTo(pub Number, pub Number);
op2!(MoveTo, "m");

#[derive(Debug)]
pub struct LineTo(pub Number, pub Number);
op2!(LineTo, "l");

#[derive(Debug)]
pub struct CubicTo(
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
);
op6!(CubicTo, "c");

#[derive(Debug)]
pub struct CubicStartTo(pub Number, pub Number, pub Number, pub Number);
op4!(CubicStartTo, "v");

#[derive(Debug)]
pub struct CubicEndTo(pub Number, pub Number, pub Number, pub Number);
op4!(CubicEndTo, "y");

#[derive(Debug)]
pub struct ClosePath;
op0!(ClosePath, "h");

#[derive(Debug)]
pub struct RectPath(pub Number, pub Number, pub Number, pub Number);
op4!(RectPath, "re");

// Path-painting operators
#[derive(Debug)]
pub struct StrokePath;
op0!(StrokePath, "S");

#[derive(Debug)]
pub struct CloseAndStrokePath;
op0!(CloseAndStrokePath, "s");

#[derive(Debug)]
pub struct FillPathNonZero;
op0!(FillPathNonZero, "f");

#[derive(Debug)]
pub struct FillPathNonZeroCompatibility;
op0!(FillPathNonZeroCompatibility, "F");

#[derive(Debug)]
pub struct FillPathEvenOdd;
op0!(FillPathEvenOdd, "f*");

#[derive(Debug)]
pub struct FillAndStrokeNonZero;
op0!(FillAndStrokeNonZero, "B");

#[derive(Debug)]
pub struct FillAndStrokeEvenOdd;
op0!(FillAndStrokeEvenOdd, "B*");

#[derive(Debug)]
pub struct CloseFillAndStrokeNonZero;
op0!(CloseFillAndStrokeNonZero, "b");

#[derive(Debug)]
pub struct CloseFillAndStrokeEvenOdd;
op0!(CloseFillAndStrokeEvenOdd, "b*");

#[derive(Debug)]
pub struct EndPath;
op0!(EndPath, "n");

// Text-showing operators

#[derive(Debug)]
pub struct ShowText<'a>(pub string::String<'a>);
op1!(ShowText<'a>, "Tj");

#[derive(Debug)]
pub struct NextLineAndShowText<'a>(pub string::String<'a>);
op1!(NextLineAndShowText<'a>, "'");

#[derive(Debug)]
pub struct ShowTextWithParameters<'a>(pub Number, pub Number, pub string::String<'a>);
op3!(ShowTextWithParameters<'a>, "\"");

#[derive(Debug)]
pub struct ShowTexts<'a>(pub Array<'a>);
op1!(ShowTexts<'a>, "TJ");

// TODO: Add remark to not collect into vector
#[derive(Debug)]
pub enum TypedOperation<'a> {
    // Compatibility operators
    BeginCompatibility(BeginCompatibility),
    EndCompatibility(EndCompatibility),

    // Graphics state operators
    SaveState(SaveState),
    RestoreState(RestoreState),
    Transform(Transform),
    LineWidth(LineWidth),
    LineCap(LineCap),
    LineJoin(LineJoin),
    MiterLimit(MiterLimit),
    DashPattern(DashPattern<'a>),
    RenderingIntent(RenderingIntent<'a>),
    FlatnessTolerance(FlatnessTolerance),
    SetGraphicsState(SetGraphicsState<'a>),

    // Path-construction operators
    MoveTo(MoveTo),
    LineTo(LineTo),
    CubicTo(CubicTo),
    CubicStartTo(CubicStartTo),
    CubicEndTo(CubicEndTo),
    ClosePath(ClosePath),
    RectPath(RectPath),

    // Path-painting operators
    StrokePath(StrokePath),
    CloseAndStrokePath(CloseAndStrokePath),
    FillPathNonZero(FillPathNonZero),
    FillPathNonZeroCompatibility(FillPathNonZeroCompatibility),
    FillPathEvenOdd(FillPathEvenOdd),
    FillAndStrokeNonZero(FillAndStrokeNonZero),
    FillAndStrokeEvenOdd(FillAndStrokeEvenOdd),
    CloseFillAndStrokeNonZero(CloseFillAndStrokeNonZero),
    CloseFillAndStrokeEvenOdd(CloseFillAndStrokeEvenOdd),
    EndPath(EndPath),

    // Text-showing operators
    ShowText(ShowText<'a>),
    NextLineAndShowText(NextLineAndShowText<'a>),
    ShowTextWithParameters(ShowTextWithParameters<'a>),
    ShowTexts(ShowTexts<'a>),

    Fallback,
}

impl<'a> TypedOperation<'a> {
    fn dispatch(operation: &Operation<'a>) -> Option<TypedOperation<'a>> {
        let op_name = operation.operator.get();
        Some(match op_name.as_ref() {
            // Compatibility operators
            b"BX" => BeginCompatibility::from_stack(&operation.operands)?.into(),
            b"EX" => EndCompatibility::from_stack(&operation.operands)?.into(),

            // Graphics state operators
            b"q" => SaveState::from_stack(&operation.operands)?.into(),
            b"cm" => Transform::from_stack(&operation.operands)?.into(),
            b"w" => LineWidth::from_stack(&operation.operands)?.into(),
            b"J" => LineCap::from_stack(&operation.operands)?.into(),
            b"j" => LineJoin::from_stack(&operation.operands)?.into(),
            b"M" => MiterLimit::from_stack(&operation.operands)?.into(),
            b"d" => DashPattern::from_stack(&operation.operands)?.into(),
            b"ri" => RenderingIntent::from_stack(&operation.operands)?.into(),
            b"i" => FlatnessTolerance::from_stack(&operation.operands)?.into(),
            b"gs" => SetGraphicsState::from_stack(&operation.operands)?.into(),

            // Path-construction operators
            b"m" => MoveTo::from_stack(&operation.operands)?.into(),
            b"l" => LineTo::from_stack(&operation.operands)?.into(),
            b"c" => CubicTo::from_stack(&operation.operands)?.into(),
            b"v" => CubicStartTo::from_stack(&operation.operands)?.into(),
            b"y" => CubicEndTo::from_stack(&operation.operands)?.into(),
            b"h" => ClosePath::from_stack(&operation.operands)?.into(),
            b"re" => RectPath::from_stack(&operation.operands)?.into(),

            // Path-painting operators
            // Clipping operators
            // Colour operators
            // Shading operator
            // XObject operator
            // Inline image operators
            // Text state operators
            // Text object operators
            // Text-positioning operators

            // Text-showing operators
            b"Tj" => ShowText::from_stack(&operation.operands)?.into(),
            b"'" => NextLineAndShowText::from_stack(&operation.operands)?.into(),
            b"\"" => ShowTextWithParameters::from_stack(&operation.operands)?.into(),
            b"TJ" => ShowTexts::from_stack(&operation.operands)?.into(),

            // Type 3 font operators
            // Marked-content operators

            // TODO: Add proper fallback
            _ => return Fallback.into(),
        })
    }
}

#[cfg(test)]
mod tests {

    // TODO: Add maaaany tests!
}
