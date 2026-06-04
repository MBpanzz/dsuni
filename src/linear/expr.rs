use crate::linear::Rational;
use crate::core::Var;

/// A term in a linear expression: `coefficient * variable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearTerm {
    pub coefficient: Rational,
    pub var: Var,
}

impl LinearTerm {
    pub fn new(coefficient: Rational, var: Var) -> Self {
        LinearTerm { coefficient, var }
    }

    /// Evaluate this term given the variable value `x`.
    pub fn eval(&self, x: Rational) -> Rational {
        self.coefficient * x
    }

    /// Negate the coefficient.
    pub fn neg(&self) -> Self {
        LinearTerm::new(-self.coefficient, self.var)
    }
}

/// A sparse linear expression: `constant + sum(coefficient_i * variable_i)`.
///
/// Terms with coefficient zero are automatically removed.
/// Adjacent terms with the same variable are merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearExpr {
    pub constant: Rational,
    pub terms: Vec<LinearTerm>, // sparse, sorted by var id, no zero-coeff terms
}

impl LinearExpr {
    /// The zero expression.
    pub const ZERO: LinearExpr = LinearExpr {
        constant: Rational::ZERO,
        terms: Vec::new(),
    };

    /// Create a constant expression.
    pub fn constant(value: Rational) -> Self {
        LinearExpr {
            constant: value,
            terms: Vec::new(),
        }
    }

    /// Create an expression from a single variable with coefficient 1.
    pub fn from_var(var: Var) -> Self {
        LinearExpr {
            constant: Rational::ZERO,
            terms: vec![LinearTerm::new(Rational::ONE, var)],
        }
    }

    /// Create an expression from a single term.
    pub fn from_term(term: LinearTerm) -> Self {
        let mut expr = LinearExpr {
            constant: Rational::ZERO,
            terms: vec![term],
        };
        expr.prune_zero_terms();
        expr
    }

    /// Number of terms (excluding the constant).
    #[inline]
    pub fn num_terms(&self) -> usize {
        self.terms.len()
    }

    /// Whether this expression is constant (no variable terms).
    #[inline]
    pub fn is_constant(&self) -> bool {
        self.terms.is_empty()
    }

    /// Evaluate the expression given a variable-to-value mapping.
    pub fn eval<F>(&self, mut get_value: F) -> Rational
    where
        F: FnMut(Var) -> Rational,
    {
        let mut sum = self.constant;
        for term in &self.terms {
            sum = sum + term.coefficient * get_value(term.var);
        }
        sum
    }

    /// Collect all variable IDs used in this expression.
    pub fn vars(&self) -> Vec<Var> {
        self.terms.iter().map(|t| t.var).collect()
    }

    /// Return the coefficient of a specific variable, or Rational::ZERO.
    pub fn coeff(&self, var: Var) -> Rational {
        for term in &self.terms {
            if term.var == var {
                return term.coefficient;
            }
        }
        Rational::ZERO
    }

    /// Negate the expression (multiply all coefficients and constant by -1).
    pub fn neg(&self) -> Self {
        let mut result = LinearExpr {
            constant: -self.constant,
            terms: self.terms.iter().map(|t| t.neg()).collect(),
        };
        result.prune_zero_terms();
        result
    }

    /// Remove terms with zero coefficient.
    fn prune_zero_terms(&mut self) {
        self.terms.retain(|t| !t.coefficient.is_zero());
    }

    /// Normalise: sort terms by var id and merge duplicates.
    pub fn normalise(&mut self) {
        if self.terms.is_empty() {
            return;
        }
        self.terms.sort_by_key(|t| t.var.id());
        let mut merged: Vec<LinearTerm> = Vec::with_capacity(self.terms.len());
        for term in self.terms.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.var == term.var {
                    last.coefficient = last.coefficient + term.coefficient;
                    continue;
                }
            }
            merged.push(term);
        }
        self.terms = merged;
        self.prune_zero_terms();
    }

    /// Is this expression exactly zero?
    pub fn is_zero(&self) -> bool {
        self.constant.is_zero() && self.terms.is_empty()
    }
}

// ── arithmetic operators ──────────────────────────────────────────────

impl std::ops::Add for LinearExpr {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        self.constant = self.constant + other.constant;
        self.terms.extend(other.terms);
        self.normalise();
        self
    }
}

impl std::ops::Sub for LinearExpr {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self + other.neg()
    }
}

impl std::ops::Mul<Rational> for LinearExpr {
    type Output = Self;
    fn mul(self, scalar: Rational) -> Self {
        let mut result = LinearExpr {
            constant: self.constant * scalar,
            terms: self
                .terms
                .into_iter()
                .map(|t| LinearTerm::new(t.coefficient * scalar, t.var))
                .collect(),
        };
        result.prune_zero_terms();
        result
    }
}

impl std::ops::Neg for LinearExpr {
    type Output = Self;
    fn neg(self) -> Self {
        self * Rational::NEG_ONE
    }
}

// ── conversions ───────────────────────────────────────────────────────

impl From<Rational> for LinearExpr {
    fn from(r: Rational) -> Self {
        LinearExpr::constant(r)
    }
}

impl From<i64> for LinearExpr {
    fn from(n: i64) -> Self {
        LinearExpr::constant(Rational::from(n))
    }
}

impl From<Var> for LinearExpr {
    fn from(var: Var) -> Self {
        LinearExpr::from_var(var)
    }
}

impl std::fmt::Display for LinearExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts: Vec<String> = Vec::new();

        // Constant term
        if !self.constant.is_zero() || self.terms.is_empty() {
            parts.push(self.constant.to_string());
        }

        for term in &self.terms {
            let coeff = term.coefficient;
            let var_str = term.var.to_string();
            if coeff == Rational::ONE {
                parts.push(var_str);
            } else if coeff == Rational::NEG_ONE {
                parts.push(format!("-{}", var_str));
            } else {
                parts.push(format!("{}*{}", coeff, var_str));
            }
        }

        if parts.is_empty() {
            write!(f, "0")
        } else {
            write!(f, "{}", parts.join(" + "))
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant() {
        let e = LinearExpr::constant(Rational::new(5, 1));
        assert!(e.is_constant());
        assert_eq!(e.to_string(), "5");
    }

    #[test]
    fn test_from_var() {
        let e = LinearExpr::from_var(Var::new(0));
        assert_eq!(e.num_terms(), 1);
        assert_eq!(e.to_string(), "x0");
    }

    #[test]
    fn test_eval() {
        let mut e = LinearExpr::from_var(Var::new(0));
        e.constant = Rational::new(1, 1);
        let val = e.eval(|v| if v == Var::new(0) { Rational::new(3, 1) } else { Rational::ZERO });
        assert_eq!(val, Rational::new(4, 1));
    }

    #[test]
    fn test_negation() {
        let e = LinearExpr::from_var(Var::new(0));
        let ne = e.neg();
        assert_eq!(ne.coeff(Var::new(0)), Rational::NEG_ONE);
    }

    #[test]
    fn test_addition() {
        let a = LinearExpr::from_var(Var::new(0));
        let b = LinearExpr::from_var(Var::new(1));
        let c = a + b;
        assert_eq!(c.num_terms(), 2);
    }

    #[test]
    fn test_merge_same_var() {
        let a = LinearExpr::from_term(LinearTerm::new(Rational::new(2, 1), Var::new(0)));
        let b = LinearExpr::from_term(LinearTerm::new(Rational::new(3, 1), Var::new(0)));
        let c = a + b;
        assert_eq!(c.num_terms(), 1);
        assert_eq!(c.coeff(Var::new(0)), Rational::new(5, 1));
    }

    #[test]
    fn test_scalar_mul() {
        let e = LinearExpr::from_var(Var::new(0)) * Rational::new(3, 1);
        assert_eq!(e.coeff(Var::new(0)), Rational::new(3, 1));
    }

    #[test]
    fn test_zero_coeff_pruned() {
        let e = LinearExpr::from_term(LinearTerm::new(Rational::ZERO, Var::new(0)));
        assert!(e.is_constant());
    }

    #[test]
    fn test_vars() {
        let e = LinearExpr::from_var(Var::new(5)) + LinearExpr::from_var(Var::new(2));
        assert_eq!(e.vars(), vec![Var::new(2), Var::new(5)]);
    }
}
