//! Linear expression normalisation pipeline.
//!
//! Transforms raw constraints into a canonical internal representation:
//!   1. Move all terms to the left-hand side (RHS = 0).
//!   2. Merge like terms.
//!   3. Fold constants.
//!   4. Reduce coefficients (divide by GCD where sensible).
//!   5. Unify inequality direction (all ≤ form with integer coefficients).

use crate::linear::{CmpOp, LinearConstraint, LinearExpr, Rational};

/// Normalise a single constraint:
///
/// - Ensures all terms are on the left side, RHS = 0
/// - Merges duplicate variable terms
/// - Folds constant terms
/// - Optionally unifies inequalities to `≤ 0` form
/// - Divides through by the GCD of all coefficients (integer constraints)
pub fn normalize_constraint(constraint: &LinearConstraint) -> LinearConstraint {
    let mut expr = constraint.expr.clone();
    expr.normalise();

    // Move everything to LHS: if op is Ge, negate both sides → Le.
    // (The constraint is always stored as expr [op] 0 after normalisation.)
    let (expr, op) = match constraint.op {
        CmpOp::Ge => (expr.neg(), CmpOp::Le),
        CmpOp::Le | CmpOp::Eq => (expr, constraint.op),
    };

    // Divide by the GCD of all coefficients (if we can find one).
    // For now, we only do this for equality constraints with all-integer coeffs.
    let expr = try_reduce(&expr);

    LinearConstraint {
        expr,
        op,
        name: constraint.name.clone(),
    }
}

/// Try to divide all coefficients and the constant by their GCD.
/// Only applies when all coefficients are integers.
fn try_reduce(expr: &LinearExpr) -> LinearExpr {
    // Collect absolute values of all coefficients and constant numerator.
    let mut values: Vec<u64> = Vec::new();

    // From the constant
    let c_num = expr.constant.num().unsigned_abs();
    if c_num > 0 {
        values.push(c_num);
    }

    // From each term coefficient
    for term in &expr.terms {
        let num = term.coefficient.num().unsigned_abs();
        if num > 0 {
            values.push(num);
        }
    }

    if values.len() < 2 {
        return expr.clone(); // nothing meaningful to reduce
    }

    // Compute GCD of all values
    let g = values.iter().copied().reduce(gcd).unwrap_or(1);
    if g <= 1 {
        return expr.clone();
    }

    // Divide through by g
    let g = g as i64;
    let new_constant = if expr.constant.num() % g == 0 {
        Rational::new(expr.constant.num() / g, expr.constant.den())
    } else {
        return expr.clone(); // can't divide evenly
    };

    let new_terms: Vec<_> = expr
        .terms
        .iter()
        .map(|t| {
            let new_coeff = if t.coefficient.num() % g == 0 {
                Rational::new(t.coefficient.num() / g, t.coefficient.den())
            } else {
                return t.clone(); // can't divide — keep original
            };
            crate::linear::LinearTerm::new(new_coeff, t.var)
        })
        .collect();

    let mut result = LinearExpr {
        constant: new_constant,
        terms: new_terms,
    };
    result.normalise();
    result
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Normalise a slice of constraints.
pub fn normalize_constraints(constraints: &[LinearConstraint]) -> Vec<LinearConstraint> {
    constraints.iter().map(normalize_constraint).collect()
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Var;

    #[test]
    fn test_le_stays_le() {
        let c = LinearConstraint::le(LinearExpr::from_var(Var::new(0)) - LinearExpr::from(5i64));
        let nc = normalize_constraint(&c);
        assert_eq!(nc.op, CmpOp::Le);
    }

    #[test]
    fn test_ge_becomes_le() {
        let c = LinearConstraint::ge(LinearExpr::from_var(Var::new(0)) - LinearExpr::from(3i64));
        let nc = normalize_constraint(&c);
        assert_eq!(nc.op, CmpOp::Le);
        // x0 - 3 >= 0 becomes -x0 + 3 <= 0
        assert_eq!(nc.expr.coeff(Var::new(0)), Rational::NEG_ONE);
        assert_eq!(nc.expr.constant, Rational::new(3, 1));
    }

    #[test]
    fn test_constant_folding() {
        let c = LinearConstraint::le(
            LinearExpr::from_var(Var::new(0))
                + LinearExpr::from_var(Var::new(0)),
        );
        let nc = normalize_constraint(&c);
        assert_eq!(nc.expr.num_terms(), 1);
        assert_eq!(nc.expr.coeff(Var::new(0)), Rational::new(2, 1));
    }

    #[test]
    fn test_coeff_reduction() {
        // 2*x0 + 4*x1 - 2 <= 0  →  x0 + 2*x1 - 1 <= 0
        let mut expr = LinearExpr::from_var(Var::new(0)) * Rational::new(2, 1);
        expr = expr + LinearExpr::from_var(Var::new(1)) * Rational::new(4, 1);
        expr = expr - LinearExpr::from(2i64);
        let c = LinearConstraint::le(expr);
        let nc = normalize_constraint(&c);
        assert_eq!(nc.expr.coeff(Var::new(0)), Rational::ONE);
        assert_eq!(nc.expr.coeff(Var::new(1)), Rational::new(2, 1));
        assert_eq!(nc.expr.constant, Rational::NEG_ONE);
    }

    #[test]
    fn test_eq_normalize() {
        let c = LinearConstraint::eq(
            LinearExpr::from_var(Var::new(0)) * Rational::new(3, 1)
                + LinearExpr::from(6i64),
        );
        let nc = normalize_constraint(&c);
        assert_eq!(nc.op, CmpOp::Eq);
        assert_eq!(nc.expr.coeff(Var::new(0)), Rational::ONE);
        assert_eq!(nc.expr.constant, Rational::new(2, 1));
    }
}
