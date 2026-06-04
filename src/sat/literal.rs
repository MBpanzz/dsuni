/// A SAT literal — a variable with a sign (positive/true or negative/false).
///
/// Variable numbering is **0-based** internally.
/// DIMACS literals (1-indexed, sign = polarity) are converted via [`Lit::from_dimacs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lit {
    /// Variable index (0-based).
    pub var: usize,
    /// `true` = variable is assigned true; `false` = variable is assigned false.
    pub sign: bool,
}

impl Lit {
    /// Create a literal from a variable and sign.
    #[inline]
    pub fn new(var: usize, sign: bool) -> Self {
        Lit { var, sign }
    }

    /// Create a positive literal (variable = true).
    #[inline]
    pub fn positive(var: usize) -> Self {
        Lit { var, sign: true }
    }

    /// Create a negative literal (variable = false).
    #[inline]
    pub fn negative(var: usize) -> Self {
        Lit { var, sign: false }
    }

    /// Convert from a DIMACS integer.
    ///
    /// DIMACS uses 1-indexed variables: `1` = x1=true, `-1` = x1=false.
    #[inline]
    pub fn from_dimacs(dimacs: i32) -> Self {
        assert!(dimacs != 0, "DIMACS literal cannot be 0");
        let var = dimacs.unsigned_abs() as usize - 1; // 1-indexed → 0-indexed
        let sign = dimacs > 0;
        Lit { var, sign }
    }

    /// Convert to a DIMACS integer (1-indexed).
    #[inline]
    pub fn to_dimacs(self) -> i32 {
        let var = (self.var + 1) as i32;
        if self.sign { var } else { -var }
    }

    /// Return the complementary literal.
    #[inline]
    pub fn not(self) -> Self {
        Lit { var: self.var, sign: !self.sign }
    }

    /// Index for watch-list and activity arrays: `2*var + sign`.
    ///
    /// Positive literal at even index, negative at odd.
    #[inline]
    pub fn index(self) -> usize {
        self.var * 2 + self.sign as usize
    }

    /// Number of index slots needed for `var_count` variables.
    #[inline]
    pub fn index_count(var_count: usize) -> usize {
        var_count * 2
    }
}

impl std::fmt::Display for Lit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.sign {
            write!(f, "x{}", self.var + 1)
        } else {
            write!(f, "~x{}", self.var + 1)
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_literal() {
        let l = Lit::positive(0);
        assert_eq!(l.var, 0);
        assert!(l.sign);
    }

    #[test]
    fn test_negative_literal() {
        let l = Lit::negative(2);
        assert_eq!(l.var, 2);
        assert!(!l.sign);
    }

    #[test]
    fn test_not() {
        let l = Lit::positive(1);
        let n = l.not();
        assert_eq!(n.var, 1);
        assert!(!n.sign);
        assert_eq!(n.not(), l);
    }

    #[test]
    fn test_dimacs_roundtrip() {
        assert_eq!(Lit::from_dimacs(5).to_dimacs(), 5);
        assert_eq!(Lit::from_dimacs(-3).to_dimacs(), -3);
        assert_eq!(Lit::from_dimacs(1).to_dimacs(), 1);
    }

    #[test]
    fn test_index() {
        // Lit.positive(0) index = 0*2 + 1 = 1
        // Lit.negative(0) index = 0*2 + 0 = 0
        assert_eq!(Lit::positive(0).index(), 1);
        assert_eq!(Lit::negative(0).index(), 0);
        assert_eq!(Lit::positive(1).index(), 3);
        assert_eq!(Lit::negative(1).index(), 2);
    }

    #[test]
    fn test_index_count() {
        assert_eq!(Lit::index_count(3), 6);
    }
}
