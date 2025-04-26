mod type2;
mod type4;

use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{DOMAIN, RANGE};
use crate::object::number::Number;
use log::{error, warn};
use smallvec::SmallVec;

type Values = SmallVec<[f32; 6]>;

impl From<Array<'_>> for Values {
    fn from(value: Array) -> Self {
        value
            .iter::<Number>()
            .map(|n| n.as_f32())
            .collect::<Values>()
    }
}

struct Clamper(Values);

impl Clamper {
    fn clamp(&self, val: f32, idx: usize) -> f32 {
        if idx * 2 >= self.0.len() {
            warn!("the domain/range of the function was exceeded");
        }

        let min = self.0.get(idx * 2).copied().unwrap_or(0.0);
        let max = self.0.get(idx * 2 + 1).copied().unwrap_or(0.0);

        val.clamp(min, max)
    }

    fn dimension(&self) -> usize {
        self.0.len() / 2
    }
}

struct CommonProperties {
    domain: Clamper,
    range: Option<Clamper>,
}

impl CommonProperties {
    #[must_use]
    fn clamp_domain(&self, input: &mut Values) -> Option<()> {
        if input.len() != self.domain.dimension() {
            error!("mismatch while clamping domain of postscript function");

            return None;
        }

        for (idx, val) in input.iter_mut().enumerate() {
            self.clamp_domain_single(val, idx);
        }

        Some(())
    }

    fn clamp_domain_single(&self, val: &mut f32, idx: usize) {
        *val = self.domain.clamp(*val, idx);
    }

    #[must_use]
    fn clamp_range(&self, input: &mut Values) -> Option<()> {
        if let Some(range) = &self.range {
            if input.len() != range.dimension() {
                error!("mismatch while clamping range of postscript function");

                return None;
            }

            for (idx, val) in input.iter_mut().enumerate() {
                *val = range.clamp(*val, idx);
            }
        }

        Some(())
    }
}

impl TryFrom<Dict<'_>> for CommonProperties {
    type Error = ();

    fn try_from(value: Dict<'_>) -> Result<Self, Self::Error> {
        let domain = value.get::<Array>(DOMAIN).map(|a| a.into()).ok_or(())?;

        let range = value.get::<Array>(RANGE).map(|a| a.into());

        Ok(CommonProperties {
            domain: Clamper(domain),
            range: range.map(|s| Clamper(s)),
        })
    }
}
