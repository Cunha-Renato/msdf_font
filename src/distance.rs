//! Code taken from https://github.com/Chlumsky/msdfgen.

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SignedDistance {
    pub(crate) distance: f64,
    pub(crate) dot: f64,
}
impl Default for SignedDistance {
    #[inline]
    fn default() -> Self {
        Self {
            distance: f64::INFINITY,
            dot: f64::NEG_INFINITY,
        }
    }
}
impl SignedDistance {
    #[inline]
    pub(crate) const fn new(distance: f64, dot: f64) -> Self {
        Self { distance, dot }
    }
}
impl PartialOrd for SignedDistance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = self.distance.abs();
        let b = other.distance.abs();

        match a.partial_cmp(&b)? {
            Ordering::Equal => self.dot.partial_cmp(&other.dot),
            ord => Some(ord),
        }
    }
}

pub(crate) struct MultiDistance {
    pub(crate) r: f64,
    pub(crate) g: f64,
    pub(crate) b: f64,
}
