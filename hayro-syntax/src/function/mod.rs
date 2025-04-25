use log::warn;
use smallvec::SmallVec;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{DOMAIN, RANGE};
use crate::object::number::Number;
use crate::object::Object;

struct Clamper(SmallVec<[f32; 6]>);

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

impl TryFrom<Dict<'_>> for CommonProperties {
    type Error = ();

    fn try_from(value: Dict<'_>) -> Result<Self, Self::Error> {
        let domain = value.get::<Array>(DOMAIN)
            .map(|a| a.iter::<Number>().map(|n| n.as_f32().unwrap()))
            .ok_or(())?
            .collect::<SmallVec<[f32; 6]>>();
        
        let range = value.get::<Array>(RANGE)
            .map(|a| a.iter::<Number>().map(|n| n.as_f32().unwrap()).collect::<SmallVec<[f32; 6]>>());
        
        Ok(CommonProperties {
            domain: Clamper(domain),
            range: range.map(|s| Clamper(s))
        })
    }
}