use crate::core::{Model, SolverError};

/// The reason behind an `Unknown` result.
#[derive(Debug, Clone, PartialEq)]
pub enum UnknownReason {
    /// The feature or theory is not yet supported.
    Unsupported,
    /// The problem is undecidable for this solver.
    Undecidable,
    /// A numerical issue prevented a decision.
    NumericalIssue,
    /// A resource limit (time/memory) was hit without a decision.
    ResourceLimit,
    /// The algorithm is incomplete for this class of problems.
    IncompleteAlgorithm,
    /// Internal reason not covered above.
    InternalReason(String),
}

impl std::fmt::Display for UnknownReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnknownReason::Unsupported => write!(f, "unsupported"),
            UnknownReason::Undecidable => write!(f, "undecidable"),
            UnknownReason::NumericalIssue => write!(f, "numerical issue"),
            UnknownReason::ResourceLimit => write!(f, "resource limit"),
            UnknownReason::IncompleteAlgorithm => write!(f, "incomplete algorithm"),
            UnknownReason::InternalReason(msg) => write!(f, "internal: {}", msg),
        }
    }
}

/// The top-level result returned by any solver.
#[derive(Debug, Clone, PartialEq)]
pub enum SolverResult {
    /// A satisfying/feasible model was found.
    Sat(Model),
    /// The problem is unsatisfiable/infeasible.
    Unsat,
    /// The solver could not determine satisfiability.
    Unknown(UnknownReason),
    /// The solver timed out.
    Timeout,
    /// The solver exceeded its memory limit.
    MemoryExceeded,
    /// An error occurred.
    Error(SolverError),
}

impl SolverResult {
    /// Was the result definitively SAT?
    pub fn is_sat(&self) -> bool {
        matches!(self, SolverResult::Sat(_))
    }

    /// Was the result definitively UNSAT?
    pub fn is_unsat(&self) -> bool {
        matches!(self, SolverResult::Unsat)
    }

    /// Was the result an error or unknown?
    pub fn is_unknown(&self) -> bool {
        matches!(self, SolverResult::Unknown(_))
    }

    /// Was the result a definitive answer (SAT or UNSAT)?
    pub fn is_conclusive(&self) -> bool {
        matches!(self, SolverResult::Sat(_) | SolverResult::Unsat)
    }
}

impl std::fmt::Display for SolverResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverResult::Sat(model) => write!(f, "SAT\n{}", model),
            SolverResult::Unsat => write!(f, "UNSAT"),
            SolverResult::Unknown(reason) => write!(f, "UNKNOWN ({})", reason),
            SolverResult::Timeout => write!(f, "TIMEOUT"),
            SolverResult::MemoryExceeded => write!(f, "MEMORY EXCEEDED"),
            SolverResult::Error(err) => write!(f, "ERROR: {}", err),
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Assignment, Model, Value, Var};

    #[test]
    fn test_sat_result() {
        let mut a = Assignment::new();
        a.set(Var::new(0), Value::Bool(true));
        let model = Model::new(a);
        let r = SolverResult::Sat(model);
        assert!(r.is_sat());
        assert!(!r.is_unsat());
        assert!(r.is_conclusive());
    }

    #[test]
    fn test_unsat_result() {
        let r = SolverResult::Unsat;
        assert!(r.is_unsat());
        assert!(r.is_conclusive());
        assert_eq!(r.to_string(), "UNSAT");
    }

    #[test]
    fn test_unknown_result() {
        let r = SolverResult::Unknown(UnknownReason::Unsupported);
        assert!(r.is_unknown());
        assert!(!r.is_conclusive());
    }

    #[test]
    fn test_timeout_result() {
        let r = SolverResult::Timeout;
        assert!(!r.is_conclusive());
        assert_eq!(r.to_string(), "TIMEOUT");
    }

    #[test]
    fn test_error_result() {
        let err = SolverError::InternalError("oops".into());
        let r = SolverResult::Error(err);
        assert!(!r.is_conclusive());
        assert!(r.to_string().contains("oops"));
    }
}
