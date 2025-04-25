mod type2;

use crate::object::Object;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{DOMAIN, RANGE};
use crate::object::number::Number;
use log::warn;
use smallvec::SmallVec;

type Values = SmallVec<[f32; 6]>;

impl From<Array<'_>> for Values {
    fn from(value: Array) -> Self {
        value
            .iter::<Number>()
            .map(|n| n.as_f32().unwrap())
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
    fn clamp_domain(&self, input: &mut Values) {
        for (idx, val) in input.iter_mut().enumerate() {
            self.clamp_domain_single(val, idx);
        }
    }

    fn clamp_domain_single(&self, val: &mut f32, idx: usize) {
        *val = self.domain.clamp(*val, idx);
    }

    fn clamp_range(&self, input: &mut Values) {
        if let Some(range) = &self.range {
            for (idx, val) in input.iter_mut().enumerate() {
                *val = range.clamp(*val, idx);
            }
        }
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
