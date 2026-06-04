//! MILP solver — branch-and-bound on LP relaxations.
//!
//! Algorithm:
//! 1. LP relaxation of the original problem
//! 2. If LP solution is integer-feasible → incumbent
//! 3. If LP has fractional integer vars → branch
//! 4. Two child nodes: x ≤ ⌊x⌋ / x ≥ ⌈x⌉
//! 5. Prune on infeasibility / bound worse than incumbent

mod solver;

pub use solver::*;

use crate::core::{Config, LpResult};
use crate::linear::LinearSystem;

/// Solve a MILP using branch and bound.
pub fn solve_milp(system: &LinearSystem, config: &Config) -> LpResult {
    let mut bnb = BnBSolver::new(config.clone());
    bnb.solve(system)
}
