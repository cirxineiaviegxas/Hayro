use crate::file::xref::XRef;
use crate::function::CommonProperties;
use crate::object::number::{InternalNumber, Number};
use crate::reader::Reader;
use log::{debug, error};

struct Type4 {
    common: CommonProperties,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostScriptOp {
    Real(f32),
    Integer(i32),
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
    If,
    IfElse,
    Copy,
    Dup,
    Exch,
    Index,
    Pop,
    Roll,
}

impl PostScriptOp {
    fn from_token(data: &[u8]) -> Option<Self> {
        let mut r = Reader::new(data);
        let op = if let Some(n) = r.read::<true, Number>(&XRef::dummy()) {
            match n.0 {
                InternalNumber::Real(r) => PostScriptOp::Real(r),
                InternalNumber::Integer(i) => PostScriptOp::Integer(i),
            }
        } else {
            match data {
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
                b"if" => Self::If,
                b"ifelse" => Self::IfElse,
                b"copy" => Self::Copy,
                b"dup" => Self::Dup,
                b"exch" => Self::Exch,
                b"index" => Self::Index,
                b"pop" => Self::Pop,
                b"roll" => Self::Roll,
                _ => {
                    error!("encountered unknown postscript operator {}", data);

                    return None;
                }
            }
        };

        Some(op)
    }
}
