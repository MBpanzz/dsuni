use crate::linear::{LinearExpr, Rational};

/// The relational operator of a linear constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    /// Equality: `expr == 0`
    Eq,
    /// Less-or-equal: `expr <= 0`
    Le,
    /// Greater-or-equal: `expr >= 0`
    Ge,
}

impl CmpOp {
    /// Flip the operator (swap ≤ and ≥, keep =).
    pub fn flip(&self) -> Self {
        match self {
            CmpOp::Eq => CmpOp::Eq,
            CmpOp::Le => CmpOp::Ge,
            CmpOp::Ge => CmpOp::Le,
        }
    }
}

/// A linear constraint of the form `expr [≤|=|≥] 0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearConstraint {
    pub expr: LinearExpr,
    pub op: CmpOp,
    /// Optional name / label.
    pub name: Option<String>,
}

impl LinearConstraint {
    /// Create a constraint with the given expression and operator.
    pub fn new(expr: LinearExpr, op: CmpOp) -> Self {
        LinearConstraint {
            expr,
            op,
            name: None,
        }
    }

    /// Create a new equality constraint: `expr == 0`.
    pub fn eq(expr: LinearExpr) -> Self {
        LinearConstraint {
            expr,
            op: CmpOp::Eq,
            name: None,
        }
    }

    /// Create a new less-or-equal constraint: `expr <= 0`.
    pub fn le(expr: LinearExpr) -> Self {
        LinearConstraint {
            expr,
            op: CmpOp::Le,
            name: None,
        }
    }

    /// Create a new greater-or-equal constraint: `expr >= 0`.
    pub fn ge(expr: LinearExpr) -> Self {
        LinearConstraint {
            expr,
            op: CmpOp::Ge,
            name: None,
        }
    }

    /// Re-express as `expr <= 0` by flipping sides if needed.
    pub fn to_le(&self) -> Self {
        match self.op {
            CmpOp::Le => self.clone(),
            CmpOp::Ge => LinearConstraint {
                expr: self.expr.neg(),
                op: CmpOp::Le,
                name: self.name.clone(),
            },
            CmpOp::Eq => self.clone(),
        }
    }

    /// Set a name for this constraint.
    pub fn named(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Evaluate whether a model satisfies this constraint.
    pub fn check<F>(&self, get_value: F) -> bool
    where
        F: FnMut(crate::core::Var) -> Rational,
    {
        let val = self.expr.eval(get_value);
        match self.op {
            CmpOp::Eq => val.is_zero(),
            CmpOp::Le => !val.is_positive(),
            CmpOp::Ge => !val.is_negative(),
        }
    }
}

impl std::fmt::Display for LinearConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op_str = match self.op {
            CmpOp::Eq => "=",
            CmpOp::Le => "<=",
            CmpOp::Ge => ">=",
        };
        if let Some(ref name) = self.name {
            write!(f, "{}: {} {} 0", name, self.expr, op_str)
        } else {
            write!(f, "{} {} 0", self.expr, op_str)
        }
    }
}

/// An objective function to be optimised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Objective {
    /// Minimise the given expression.
    Minimise(LinearExpr),
    /// Maximise the given expression.
    Maximise(LinearExpr),
    /// No objective (feasibility problem).
    None,
}

impl Objective {
    /// Is this a feasibility problem (no objective)?
    pub fn is_feasibility(&self) -> bool {
        matches!(self, Objective::None)
    }

    /// Return the objective expression, if any.
    pub fn expr(&self) -> Option<&LinearExpr> {
        match self {
            Objective::Minimise(e) | Objective::Maximise(e) => Some(e),
            Objective::None => None,
        }
    }
}

impl std::fmt::Display for Objective {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Objective::Minimise(expr) => write!(f, "minimise {}", expr),
            Objective::Maximise(expr) => write!(f, "maximise {}", expr),
            Objective::None => write!(f, "feasibility"),
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Var;

    #[test]
    fn test_eq_constraint() {
        let c = LinearConstraint::eq(LinearExpr::from_var(Var::new(0)));
        assert_eq!(c.op, CmpOp::Eq);
    }

    #[test]
    fn test_le_constraint() {
        let c = LinearConstraint::le(LinearExpr::from_var(Var::new(0)));
        assert_eq!(c.op, CmpOp::Le);
    }

    #[test]
    fn test_ge_to_le() {
        let c = LinearConstraint::ge(LinearExpr::from_var(Var::new(0)));
        let le = c.to_le();
        assert_eq!(le.op, CmpOp::Le);
        // x0 >= 0 → -x0 <= 0
        assert_eq!(le.expr.coeff(Var::new(0)), Rational::NEG_ONE);
    }

    #[test]
    fn test_check_eq() {
        let c = LinearConstraint::eq(LinearExpr::constant(Rational::ZERO));
        assert!(c.check(|_| Rational::ZERO));
    }

    #[test]
    fn test_check_le() {
        let c = LinearConstraint::le(LinearExpr::constant(Rational::new(-3, 1)));
        assert!(c.check(|_| Rational::ZERO));
    }

    #[test]
    fn test_check_ge_false() {
        let c = LinearConstraint::ge(LinearExpr::constant(Rational::new(-1, 1)));
        // -1 >= 0 is false
        assert!(!c.check(|_| Rational::ZERO));
    }

    #[test]
    fn test_objective() {
        let obj = Objective::Minimise(LinearExpr::from_var(Var::new(0)));
        assert!(!obj.is_feasibility());
        assert!(obj.expr().is_some());

        assert!(Objective::None.is_feasibility());
    }
}
