use crate::file::xref::XRef;
use crate::function::CommonProperties;
use crate::object::number::{InternalNumber, Number};
use crate::reader::Reader;
use crate::{OptionLog, content};
use log::{debug, error};
use smallvec::{SmallVec, smallvec};

type ParseStack = SmallVec<[Vec<PostScriptOp>; 2]>;

struct Type4 {
    common: CommonProperties,
}

fn parse_procedure(data: &[u8]) -> Option<Vec<PostScriptOp>> {
    let mut r = Reader::new(data);
    parse_procedure_inner(&mut r).warn_none("failed to read postscript program")
}

fn parse_procedure_inner(r: &mut Reader) -> Option<Vec<PostScriptOp>> {
    let mut stack: ParseStack = smallvec![];

    let mut ops = vec![];
    r.skip_white_spaces_and_comments();
    r.forward_tag(b"{")?;

    loop {
        r.skip_white_spaces_and_comments();

        if r.peek_byte()? == b'}' {
            r.forward_tag(b"}")?;

            break;
        } else if r.peek_byte()? == b'{' {
            stack.push(parse_procedure_inner(r)?);
        } else {
            let op = PostScriptOp::from_reader(r, &mut stack)?;
            ops.push(op);
        }
    }

    Some(ops)
}

#[derive(Debug, Clone, PartialEq)]
pub enum PostScriptOp {
    Number(Number),
    Abs,
    Add,
    Atan,
    Ceiling,
    Cos,
    Cvi,
    Cvr,
    Div,
    Exp,
    Floor,
    Idiv,
    Ln,
    Log,
    Mod,
    Mul,
    Neg,
    Round,
    Sin,
    Sqrt,
    Sub,
    Truncate,
    And,
    Bitshift,
    Eq,
    False,
    Ge,
    Gt,
    Le,
    Lt,
    Ne,
    Not,
    Or,
    True,
    Xor,
    If(Vec<PostScriptOp>),
    IfElse(Vec<PostScriptOp>, Vec<PostScriptOp>),
    Copy,
    Dup,
    Exch,
    Index,
    Pop,
    Roll,
}

impl PostScriptOp {
    fn from_reader(r: &mut Reader, stack: &mut ParseStack) -> Option<Self> {
        let op = if let Some(n) = r.read::<true, Number>(&XRef::dummy()) {
            // TODO: Support radix numbers
            Self::Number(n)
        } else {
            let op = r.read::<true, content::Operator>(&XRef::dummy())?.get();
            match op.as_ref() {
                b"abs" => Self::Abs,
                b"add" => Self::Add,
                b"atan" => Self::Atan,
                b"ceiling" => Self::Ceiling,
                b"cos" => Self::Cos,
                b"cvi" => Self::Cvi,
                b"cvr" => Self::Cvr,
                b"div" => Self::Div,
                b"exp" => Self::Exp,
                b"floor" => Self::Floor,
                b"idiv" => Self::Idiv,
                b"ln" => Self::Ln,
                b"log" => Self::Log,
                b"mod" => Self::Mod,
                b"mul" => Self::Mul,
                b"neg" => Self::Neg,
                b"round" => Self::Round,
                b"sin" => Self::Sin,
                b"sqrt" => Self::Sqrt,
                b"sub" => Self::Sub,
                b"truncate" => Self::Truncate,
                b"and" => Self::And,
                b"bitshift" => Self::Bitshift,
                b"eq" => Self::Eq,
                b"false" => Self::False,
                b"ge" => Self::Ge,
                b"gt" => Self::Gt,
                b"le" => Self::Le,
                b"lt" => Self::Lt,
                b"ne" => Self::Ne,
                b"not" => Self::Not,
                b"or" => Self::Or,
                b"true" => Self::True,
                b"xor" => Self::Xor,
                b"if" => Self::If(stack.pop()?),
                b"ifelse" => {
                    let s = stack.pop()?;
                    let f = stack.pop()?;
                    Self::IfElse(f, s)
                }
                b"copy" => Self::Copy,
                b"dup" => Self::Dup,
                b"exch" => Self::Exch,
                b"index" => Self::Index,
                b"pop" => Self::Pop,
                b"roll" => Self::Roll,
                _ => {
                    error!("encountered unknown postscript operator {:?}", op);

                    return None;
                }
            }
        };

        Some(op)
    }
}

#[cfg(test)]
mod tests {
    use crate::function::type4::{PostScriptOp, parse_procedure};
    use crate::object::number::Number;

    #[test]
    fn lex_1() {
        let program = b"{ copy dup 0.4545 exch roll }";
        let parsed = parse_procedure(program).unwrap();

        assert_eq!(
            parsed,
            vec![
                PostScriptOp::Copy,
                PostScriptOp::Dup,
                PostScriptOp::Number(Number::from_f32(0.4545)),
                PostScriptOp::Exch,
                PostScriptOp::Roll,
            ]
        )
    }

    #[test]
    fn lex_3() {
        let program = b" {  {dup exch} if {0} {1} ifelse }";
        let parsed = parse_procedure(program).unwrap();

        assert_eq!(
            parsed,
            vec![
                PostScriptOp::If(vec![PostScriptOp::Dup, PostScriptOp::Exch]),
                PostScriptOp::IfElse(
                    vec![PostScriptOp::Number(Number::from_i32(0))],
                    vec![PostScriptOp::Number(Number::from_i32(1))]
                )
            ]
        )
    }
}
