//! CP solver — constraint propagation with backtracking search.
//!
//! Architecture:
//!   Variable domains → Propagation queue → Fixed point
//!     → Backtracking search (variable/value selection)
//!     → Propagate again → Solution / Failure

mod constraints;
mod dom;
mod propagator;
mod search;

// pub use constraints::*;
pub use dom::*;
pub use propagator::*;
pub use search::*;

use crate::core::{Config, SolverResult};

/// Solve a CP problem.
pub fn solve_cp(problem: &CpProblem, _config: &Config) -> SolverResult {
    let mut solver = CpSolver::new();
    solver.load(problem);
    solver.solve()
}

/// A CP problem definition.
#[derive(Debug, Clone)]
pub struct CpProblem {
    pub var_domains: Vec<(String, std::ops::RangeInclusive<i64>)>,
    pub constraints: Vec<CpConstraint>,
}

/// A CP constraint.
#[derive(Debug, Clone)]
pub enum CpConstraint {
    Eq(usize, usize),             // x = y
    Neq(usize, usize),            // x ≠ y
    Lt(usize, usize),             // x < y
    Le(usize, usize, i64),       // x ≤ y + c
    Linear(Vec<(i64, usize)>, i64, CmpRel), // Σ(ai*xi) ≤ b / = b / ≥ b
    AllDifferent(Vec<usize>),     // all x[i] ≠ x[j] for i≠j
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpRel { Le, Eq, Ge }
