use crate::core::Term;

/// A constraint in the solver's constraint store.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// The constraint term.
    pub term: Term,
    /// Whether this constraint must always hold.
    pub is_hard: bool,
    /// Optional weight for soft constraints.
    pub weight: Option<f64>,
}

impl Constraint {
    /// Create a new hard constraint.
    pub fn hard(term: Term) -> Self {
        Self {
            term,
            is_hard: true,
            weight: None,
        }
    }

    /// Create a new soft constraint with the given weight.
    pub fn soft(term: Term, weight: f64) -> Self {
        Self {
            term,
            is_hard: false,
            weight: Some(weight),
        }
    }
}

impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_hard {
            write!(f, "(hard {})", self.term)
        } else {
            write!(
                f,
                "(soft {} weight={})",
                self.term,
                self.weight.unwrap_or(1.0)
            )
        }
    }
}
