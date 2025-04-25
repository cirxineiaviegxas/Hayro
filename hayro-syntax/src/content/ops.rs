// Compatibility operators

use crate::content::{Operation, OperatorTrait, Stack};
use crate::object::array::Array;
use crate::object::name::Name;
use crate::object::number::Number;
use crate::object::string;

use crate::{op0, op1, op2, op3, op4, op6};
use log::warn;

include!("ops_generated.rs");
