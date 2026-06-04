use crate::core::{Config, SolverResult};

/// A literal-like value used for assumptions in incremental solving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralLike {
    Pos(u32),
    Neg(u32),
}

/// The common trait that every solver implements.
///
/// Provides the incremental solving interface (`push`/`pop`/`assume`/`reset`)
/// alongside the primary `solve` method.
pub trait Solver {
    /// Push a new assertion scope.
    fn push(&mut self);

    /// Pop the last assertion scope.
    fn pop(&mut self);

    /// Assume a literal for the next solve call.
    fn assume(&mut self, assumptions: &[LiteralLike]);

    /// Solve the current set of constraints.
    fn solve(&mut self) -> SolverResult;

    /// Reset the solver to its initial state.
    fn reset(&mut self);

    /// Return the solver configuration.
    fn config(&self) -> &Config;
}

/// Specialised result type for LP solvers.
#[derive(Debug, Clone, PartialEq)]
pub enum LpResult {
    /// Optimal solution found.
    Optimal(crate::core::Model, f64),
    /// Feasible solution found (no objective, or not proven optimal).
    Feasible(crate::core::Model),
    /// Problem is infeasible.
    Infeasible,
    /// Problem is unbounded.
    Unbounded,
    /// Result unknown.
    Unknown(crate::core::UnknownReason),
}

impl LpResult {
    pub fn is_feasible(&self) -> bool {
        matches!(self, LpResult::Optimal(_, _) | LpResult::Feasible(_))
    }
}

impl std::fmt::Display for LpResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LpResult::Optimal(model, obj) => write!(f, "OPTIMAL (obj={})\n{}", obj, model),
            LpResult::Feasible(model) => write!(f, "FEASIBLE\n{}", model),
            LpResult::Infeasible => write!(f, "INFEASIBLE"),
            LpResult::Unbounded => write!(f, "UNBOUNDED"),
            LpResult::Unknown(reason) => write!(f, "UNKNOWN ({})", reason),
        }
    }
}
