use crate::linear::Rational;
use crate::core::Var;

/// A variable bound: `lower <= x <= upper`.
///
/// Unbounded sides are represented with `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bound {
    pub lower: Option<Rational>,
    pub upper: Option<Rational>,
}

impl Bound {
    /// Create an unconstrained bound (no lower or upper limit).
    pub const FREE: Bound = Bound {
        lower: None,
        upper: None,
    };

    /// Create a bound with both lower and upper limits.
    pub fn new(lower: Option<Rational>, upper: Option<Rational>) -> Self {
        Bound { lower, upper }
    }

    /// Create a bound with only a lower limit.
    pub fn lower(lb: Rational) -> Self {
        Bound {
            lower: Some(lb),
            upper: None,
        }
    }

    /// Create a bound with only an upper limit.
    pub fn upper(ub: Rational) -> Self {
        Bound {
            lower: None,
            upper: Some(ub),
        }
    }

    /// Create a fixed-value bound (variable fixed to a single value).
    pub fn fixed(value: Rational) -> Self {
        Bound {
            lower: Some(value),
            upper: Some(value),
        }
    }

    /// Check whether a value satisfies this bound.
    pub fn contains(&self, value: Rational) -> bool {
        if let Some(lb) = self.lower {
            if value < lb {
                return false;
            }
        }
        if let Some(ub) = self.upper {
            if value > ub {
                return false;
            }
        }
        true
    }

    /// Is the variable fixed to a single value?
    pub fn is_fixed(&self) -> bool {
        self.lower.is_some() && self.upper.is_some() && self.lower == self.upper
    }

    /// Tighten this bound by intersecting with another bound.
    /// Returns `None` if the intersection is empty.
    pub fn intersect(&self, other: &Bound) -> Option<Bound> {
        let new_lower = match (self.lower, other.lower) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        let new_upper = match (self.upper, other.upper) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        // Check consistency
        if let (Some(lb), Some(ub)) = (new_lower, new_upper) {
            if lb > ub {
                return None;
            }
        }
        Some(Bound::new(new_lower, new_upper))
    }
}

impl std::fmt::Display for Bound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.lower, self.upper) {
            (None, None) => write!(f, "free"),
            (Some(lb), None) => write!(f, ">={}", lb),
            (None, Some(ub)) => write!(f, "<={}", ub),
            (Some(lb), Some(ub)) if lb == ub => write!(f, "={}", lb),
            (Some(lb), Some(ub)) => write!(f, "[{}, {}]", lb, ub),
        }
    }
}

/// The domain of a variable: its type (integer or real) and bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Domain {
    pub var: Var,
    pub is_integer: bool,
    pub bound: Bound,
}

impl Domain {
    pub fn new(var: Var) -> Self {
        Domain {
            var,
            is_integer: false,
            bound: Bound::FREE,
        }
    }

    pub fn integer(var: Var) -> Self {
        Domain {
            var,
            is_integer: true,
            bound: Bound::FREE,
        }
    }

    pub fn real(var: Var) -> Self {
        Domain {
            var,
            is_integer: false,
            bound: Bound::FREE,
        }
    }

    pub fn with_bound(mut self, bound: Bound) -> Self {
        self.bound = bound;
        self
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = if self.is_integer { "Int" } else { "Real" };
        write!(f, "{}::{} [{}]", self.var, ty, self.bound)
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_bound() {
        let b = Bound::FREE;
        assert!(b.contains(Rational::new(100, 1)));
        assert!(b.contains(Rational::new(-100, 1)));
    }

    #[test]
    fn test_lower_bound() {
        let b = Bound::lower(Rational::new(0, 1));
        assert!(b.contains(Rational::new(5, 1)));
        assert!(b.contains(Rational::ZERO));
        assert!(!b.contains(Rational::new(-1, 1)));
    }

    #[test]
    fn test_upper_bound() {
        let b = Bound::upper(Rational::new(10, 1));
        assert!(b.contains(Rational::new(5, 1)));
        assert!(!b.contains(Rational::new(11, 1)));
    }

    #[test]
    fn test_fixed_bound() {
        let b = Bound::fixed(Rational::new(5, 1));
        assert!(b.is_fixed());
        assert!(b.contains(Rational::new(5, 1)));
        assert!(!b.contains(Rational::new(4, 1)));
    }

    #[test]
    fn test_intersect() {
        let a = Bound::new(Some(Rational::new(0, 1)), Some(Rational::new(10, 1)));
        let b = Bound::new(Some(Rational::new(5, 1)), Some(Rational::new(15, 1)));
        let c = a.intersect(&b).unwrap();
        assert_eq!(c.lower, Some(Rational::new(5, 1)));
        assert_eq!(c.upper, Some(Rational::new(10, 1)));
    }

    #[test]
    fn test_intersect_infeasible() {
        let a = Bound::new(Some(Rational::new(0, 1)), Some(Rational::new(5, 1)));
        let b = Bound::new(Some(Rational::new(10, 1)), Some(Rational::new(15, 1)));
        assert!(a.intersect(&b).is_none());
    }

    #[test]
    fn test_domain() {
        let d = Domain::integer(Var::new(0)).with_bound(Bound::new(
            Some(Rational::ZERO),
            Some(Rational::new(10, 1)),
        ));
        assert!(d.is_integer);
        assert!(d.bound.contains(Rational::new(5, 1)));
    }
}
