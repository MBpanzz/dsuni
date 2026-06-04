/// A finite integer domain.
#[derive(Debug, Clone)]
pub struct IntDomain {
    min: i64,
    max: i64,
}

impl IntDomain {
    pub fn new(min: i64, max: i64) -> Self {
        IntDomain { min, max }
    }

    pub fn single(value: i64) -> Self {
        IntDomain::new(value, value)
    }

    #[inline]
    pub fn min(&self) -> i64 { self.min }

    #[inline]
    pub fn max(&self) -> i64 { self.max }

    #[inline]
    pub fn is_empty(&self) -> bool { self.min > self.max }

    #[inline]
    pub fn is_fixed(&self) -> bool { self.min == self.max }

    pub fn value(&self) -> Option<i64> {
        if self.is_fixed() { Some(self.min) } else { None }
    }

    #[inline]
    pub fn size(&self) -> u64 {
        if self.is_empty() { 0 } else { (self.max - self.min + 1) as u64 }
    }

    /// Remove values less than `lb`. Returns false if domain becomes empty.
    pub fn prune_lb(&mut self, lb: i64) -> bool {
        if lb > self.min {
            self.min = lb;
        }
        !self.is_empty()
    }

    /// Remove values greater than `ub`. Returns false if domain becomes empty.
    pub fn prune_ub(&mut self, ub: i64) -> bool {
        if ub < self.max {
            self.max = ub;
        }
        !self.is_empty()
    }

    /// Remove a specific value. Returns true if still valid.
    /// For interior values, removal cannot be represented in this
    /// interval-only domain; we return true optimistically.
    pub fn remove(&mut self, val: i64) -> bool {
        if val == self.min {
            self.min += 1;
        } else if val == self.max {
            self.max -= 1;
        }
        // Interior removals are not representable — sound but may miss pruning.
        !self.is_empty()
    }

    pub fn contains(&self, val: i64) -> bool {
        val >= self.min && val <= self.max
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_domain() {
        let d = IntDomain::new(0, 10);
        assert_eq!(d.min(), 0);
        assert_eq!(d.max(), 10);
    }

    #[test]
    fn test_single() {
        let d = IntDomain::single(5);
        assert!(d.is_fixed());
        assert_eq!(d.value(), Some(5));
    }

    #[test]
    fn test_prune_lb() {
        let mut d = IntDomain::new(0, 10);
        assert!(d.prune_lb(3));
        assert_eq!(d.min(), 3);
    }

    #[test]
    fn test_prune_ub() {
        let mut d = IntDomain::new(0, 10);
        assert!(d.prune_ub(7));
        assert_eq!(d.max(), 7);
    }

    #[test]
    fn test_prune_noop_still_valid() {
        let mut d = IntDomain::new(5, 10);
        assert!(d.prune_lb(3)); // lb=3 < min=5 → no change, but still valid
        assert!(d.prune_ub(15)); // ub=15 > max=10 → no change, but still valid
    }
}
