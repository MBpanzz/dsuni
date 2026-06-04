/// Interval arithmetic for bound propagation.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lb: f64,
    pub ub: f64,
}

impl Interval {
    pub fn new(lb: f64, ub: f64) -> Self {
        Interval { lb, ub }
    }

    pub fn is_empty(&self) -> bool {
        self.lb > self.ub
    }

    pub fn contains(&self, x: f64) -> bool {
        x >= self.lb && x <= self.ub
    }

    /// Intersection.
    pub fn intersect(&self, other: &Interval) -> Interval {
        Interval::new(self.lb.max(other.lb), self.ub.min(other.ub))
    }

    /// Width.
    pub fn width(&self) -> f64 {
        (self.ub - self.lb).max(0.0)
    }
}

// ── arithmetic ────────────────────────────────────────────────────────

pub fn add(a: &Interval, b: &Interval) -> Interval {
    Interval::new(a.lb + b.lb, a.ub + b.ub)
}

pub fn sub(a: &Interval, b: &Interval) -> Interval {
    Interval::new(a.lb - b.ub, a.ub - b.lb)
}

pub fn mul(a: &Interval, b: &Interval) -> Interval {
    let vals = [a.lb * b.lb, a.lb * b.ub, a.ub * b.lb, a.ub * b.ub];
    Interval::new(
        vals.iter().cloned().fold(f64::INFINITY, f64::min),
        vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    )
}

pub fn sqr(a: &Interval) -> Interval {
    if a.lb >= 0.0 {
        Interval::new(a.lb * a.lb, a.ub * a.ub)
    } else if a.ub <= 0.0 {
        Interval::new(a.ub * a.ub, a.lb * a.lb)
    } else {
        Interval::new(0.0, (a.lb * a.lb).max(a.ub * a.ub))
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let a = Interval::new(1.0, 3.0);
        let b = Interval::new(2.0, 5.0);
        let r = add(&a, &b);
        assert!((r.lb - 3.0).abs() < 1e-10);
        assert!((r.ub - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul() {
        let a = Interval::new(-2.0, 3.0);
        let b = Interval::new(1.0, 4.0);
        let r = mul(&a, &b);
        assert!((r.lb - (-8.0)).abs() < 1e-10);
        assert!((r.ub - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqr() {
        let a = Interval::new(-3.0, 2.0);
        let r = sqr(&a);
        assert!((r.lb - 0.0).abs() < 1e-10);
        assert!((r.ub - 9.0).abs() < 1e-10);
    }
}
