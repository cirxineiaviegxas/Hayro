pub mod cff;
mod charset;
mod charstring;
mod dict;
mod encoding;
mod index;
mod parser;
mod std_names;

use core::convert::TryFrom;

use parser::FromData;

pub use cff::Table;
use crate::OutlineError;
use crate::util::TryNumFrom;

/// A type-safe wrapper for string ID.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
pub(crate) struct StringId(u16);

impl FromData for StringId {
    const SIZE: usize = 2;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        u16::parse(data).map(StringId)
    }
}

trait IsEven {
    fn is_even(&self) -> bool;
    fn is_odd(&self) -> bool;
}

impl IsEven for usize {
    #[inline]
    fn is_even(&self) -> bool {
        (*self) & 1 == 0
    }

    #[inline]
    fn is_odd(&self) -> bool {
        !self.is_even()
    }
}

fn f32_abs(n: f32) -> f32 {
    n.abs()
}

#[inline]
fn conv_subroutine_index(index: f32, bias: u16) -> Result<u32, OutlineError> {
    conv_subroutine_index_impl(index, bias).ok_or(OutlineError::InvalidSubroutineIndex)
}

#[inline]
fn conv_subroutine_index_impl(index: f32, bias: u16) -> Option<u32> {
    let index = i32::try_num_from(index)?;
    let bias = i32::from(bias);

    let index = index.checked_add(bias)?;
    u32::try_from(index).ok()
}

// Adobe Technical Note #5176, Chapter 16 "Local / Global Subrs INDEXes"
#[inline]
fn calc_subroutine_bias(len: u32) -> u16 {
    if len < 1240 {
        107
    } else if len < 33900 {
        1131
    } else {
        32768
    }
}
