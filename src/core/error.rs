/// Errors that can occur during solving.
#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    /// Input could not be parsed.
    ParseError(String),
    /// Type error in the input.
    TypeError(String),
    /// The constraint is invalid or malformed.
    InvalidConstraint(String),
    /// The requested feature is not supported.
    Unsupported(String),
    /// Solver exceeded the time limit.
    Timeout,
    /// Solver exceeded the memory limit.
    MemoryExceeded,
    /// Numerical error (e.g. division by zero, overflow).
    NumericalError(String),
    /// Internal solver error.
    InternalError(String),
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverError::ParseError(msg) => write!(f, "parse error: {}", msg),
            SolverError::TypeError(msg) => write!(f, "type error: {}", msg),
            SolverError::InvalidConstraint(msg) => write!(f, "invalid constraint: {}", msg),
            SolverError::Unsupported(feature) => write!(f, "unsupported: {}", feature),
            SolverError::Timeout => write!(f, "timeout"),
            SolverError::MemoryExceeded => write!(f, "memory limit exceeded"),
            SolverError::NumericalError(msg) => write!(f, "numerical error: {}", msg),
            SolverError::InternalError(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for SolverError {}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let err = SolverError::ParseError("unexpected token".into());
        assert_eq!(err.to_string(), "parse error: unexpected token");
    }

    #[test]
    fn test_type_error_display() {
        let err = SolverError::TypeError("expected Bool".into());
        assert_eq!(err.to_string(), "type error: expected Bool");
    }

    #[test]
    fn test_timeout_display() {
        let err = SolverError::Timeout;
        assert_eq!(err.to_string(), "timeout");
    }

    #[test]
    fn test_internal_error_display() {
        let err = SolverError::InternalError("bug".into());
        assert_eq!(err.to_string(), "internal error: bug");
    }

    #[test]
    fn test_error_is_send() {
        // Ensure SolverError implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<SolverError>();
        assert_sync::<SolverError>();
    }
}
