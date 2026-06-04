use crate::core::{Sort, Var};

/// A term in the solver's expression language.
///
/// `Term` is designed as a recursive AST that covers all solver domains.
/// Initial support covers Boolean, integer, and real expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// Boolean constant.
    Bool(bool),
    /// Integer constant.
    Int(i64),
    /// Real (floating-point) constant.
    Real(f64),
    /// Variable reference.
    Var(Var),
    /// Logical negation.
    Not(Box<Term>),
    /// Logical conjunction.
    And(Vec<Term>),
    /// Logical disjunction.
    Or(Vec<Term>),
    /// Logical implication.
    Implies(Box<Term>, Box<Term>),
    /// Equality.
    Eq(Box<Term>, Box<Term>),
    /// Less-than.
    Lt(Box<Term>, Box<Term>),
    /// Less-than-or-equal.
    Le(Box<Term>, Box<Term>),
    /// Greater-than.
    Gt(Box<Term>, Box<Term>),
    /// Greater-than-or-equal.
    Ge(Box<Term>, Box<Term>),
    /// Addition.
    Add(Vec<Term>),
    /// Subtraction.
    Sub(Box<Term>, Box<Term>),
    /// Multiplication.
    Mul(Box<Term>, Box<Term>),
}

impl Term {
    /// Attempt to infer the sort of this term.
    ///
    /// Returns `None` when the sort cannot be determined (e.g. type error).
    pub fn sort(&self) -> Option<Sort> {
        match self {
            Term::Bool(_) => Some(Sort::Bool),
            Term::Int(_) => Some(Sort::Int),
            Term::Real(_) => Some(Sort::Real),
            Term::Var(_) => None, // requires a symbol table
            Term::Not(t) => t.sort().and_then(|s| {
                if s == Sort::Bool {
                    Some(Sort::Bool)
                } else {
                    None
                }
            }),
            Term::And(ts) | Term::Or(ts) => {
                if ts.iter().all(|t| t.sort() == Some(Sort::Bool)) {
                    Some(Sort::Bool)
                } else {
                    None
                }
            }
            Term::Implies(a, b) => {
                if a.sort() == Some(Sort::Bool) && b.sort() == Some(Sort::Bool) {
                    Some(Sort::Bool)
                } else {
                    None
                }
            }
            Term::Eq(a, b) => {
                if a.sort() == b.sort() {
                    Some(Sort::Bool)
                } else {
                    None
                }
            }
            Term::Lt(_, _)
            | Term::Le(_, _)
            | Term::Gt(_, _)
            | Term::Ge(_, _) => Some(Sort::Bool),
            Term::Add(ts) => {
                if ts.is_empty() {
                    return None;
                }
                let s = ts[0].sort()?;
                if s == Sort::Real || s == Sort::Int {
                    Some(s)
                } else {
                    None
                }
            }
            Term::Sub(a, b) | Term::Mul(a, b) => {
                let sa = a.sort()?;
                let sb = b.sort()?;
                if sa == sb && (sa == Sort::Real || sa == Sort::Int) {
                    Some(sa)
                } else {
                    None
                }
            }
        }
    }
}

// ── convenience constructors ──────────────────────────────────────────

impl From<bool> for Term {
    fn from(b: bool) -> Self {
        Term::Bool(b)
    }
}

impl From<i64> for Term {
    fn from(i: i64) -> Self {
        Term::Int(i)
    }
}

impl From<f64> for Term {
    fn from(f: f64) -> Self {
        Term::Real(f)
    }
}

impl From<Var> for Term {
    fn from(v: Var) -> Self {
        Term::Var(v)
    }
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Bool(b) => write!(f, "{}", b),
            Term::Int(i) => write!(f, "{}", i),
            Term::Real(x) => write!(f, "{}", x),
            Term::Var(v) => write!(f, "{}", v),
            Term::Not(t) => write!(f, "(not {})", t),
            Term::And(ts) => {
                write!(f, "(and")?;
                for t in ts {
                    write!(f, " {}", t)?;
                }
                write!(f, ")")
            }
            Term::Or(ts) => {
                write!(f, "(or")?;
                for t in ts {
                    write!(f, " {}", t)?;
                }
                write!(f, ")")
            }
            Term::Implies(a, b) => write!(f, "(=> {} {})", a, b),
            Term::Eq(a, b) => write!(f, "(= {} {})", a, b),
            Term::Lt(a, b) => write!(f, "(< {} {})", a, b),
            Term::Le(a, b) => write!(f, "(<= {} {})", a, b),
            Term::Gt(a, b) => write!(f, "(> {} {})", a, b),
            Term::Ge(a, b) => write!(f, "(>= {} {})", a, b),
            Term::Add(ts) => {
                write!(f, "(+")?;
                for t in ts {
                    write!(f, " {}", t)?;
                }
                write!(f, ")")
            }
            Term::Sub(a, b) => write!(f, "(- {} {})", a, b),
            Term::Mul(a, b) => write!(f, "(* {} {})", a, b),
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_constant() {
        let t = Term::Bool(true);
        assert_eq!(t.sort(), Some(Sort::Bool));
        assert_eq!(t.to_string(), "true");
    }

    #[test]
    fn test_int_constant() {
        let t = Term::Int(42);
        assert_eq!(t.sort(), Some(Sort::Int));
        assert_eq!(t.to_string(), "42");
    }

    #[test]
    fn test_real_constant() {
        let t = Term::Real(3.14);
        assert_eq!(t.sort(), Some(Sort::Real));
    }

    #[test]
    fn test_var_term() {
        let v = Var::new(0);
        let t: Term = v.into();
        assert_eq!(t.sort(), None); // no symbol table
        assert_eq!(t.to_string(), "x0");
    }

    #[test]
    fn test_not_bool() {
        let t = Term::Not(Box::new(Term::Bool(true)));
        assert_eq!(t.sort(), Some(Sort::Bool));
        assert_eq!(t.to_string(), "(not true)");
    }

    #[test]
    fn test_not_non_bool() {
        let t = Term::Not(Box::new(Term::Int(5)));
        assert_eq!(t.sort(), None);
    }

    #[test]
    fn test_and_all_bool() {
        let t = Term::And(vec![Term::Bool(true), Term::Bool(false)]);
        assert_eq!(t.sort(), Some(Sort::Bool));
    }

    #[test]
    fn and_mixed_sort_returns_none() {
        let t = Term::And(vec![Term::Bool(true), Term::Int(1)]);
        assert_eq!(t.sort(), None);
    }

    #[test]
    fn test_implies_bool() {
        let t = Term::Implies(
            Box::new(Term::Bool(true)),
            Box::new(Term::Bool(false)),
        );
        assert_eq!(t.sort(), Some(Sort::Bool));
        assert_eq!(t.to_string(), "(=> true false)");
    }

    #[test]
    fn test_eq_same_sort() {
        let t = Term::Eq(Box::new(Term::Int(1)), Box::new(Term::Int(2)));
        assert_eq!(t.sort(), Some(Sort::Bool));
    }

    #[test]
    fn test_eq_different_sort() {
        let t = Term::Eq(Box::new(Term::Int(1)), Box::new(Term::Bool(true)));
        assert_eq!(t.sort(), None);
    }

    #[test]
    fn test_arithmetic_sort() {
        let add = Term::Add(vec![Term::Int(1), Term::Int(2)]);
        assert_eq!(add.sort(), Some(Sort::Int));
        assert_eq!(add.to_string(), "(+ 1 2)");

        let sub = Term::Sub(Box::new(Term::Real(1.0)), Box::new(Term::Real(2.0)));
        assert_eq!(sub.sort(), Some(Sort::Real));

        let mul = Term::Mul(Box::new(Term::Int(3)), Box::new(Term::Int(4)));
        assert_eq!(mul.sort(), Some(Sort::Int));
        assert_eq!(mul.to_string(), "(* 3 4)");
    }

    #[test]
    fn test_comparison_sort() {
        for cmp in [
            Term::Lt(Box::new(Term::Int(1)), Box::new(Term::Int(2))),
            Term::Le(Box::new(Term::Int(1)), Box::new(Term::Int(2))),
            Term::Gt(Box::new(Term::Int(1)), Box::new(Term::Int(2))),
            Term::Ge(Box::new(Term::Int(1)), Box::new(Term::Int(2))),
        ] {
            assert_eq!(cmp.sort(), Some(Sort::Bool));
        }
    }

    #[test]
    fn test_from_traits() {
        assert_eq!(Term::from(true), Term::Bool(true));
        assert_eq!(Term::from(42i64), Term::Int(42));
        assert_eq!(Term::from(1.5f64), Term::Real(1.5));
        assert_eq!(Term::from(Var::new(7)), Term::Var(Var::new(7)));
    }

    #[test]
    fn test_add_empty_returns_none() {
        let t = Term::Add(vec![]);
        assert_eq!(t.sort(), None);
    }
}
