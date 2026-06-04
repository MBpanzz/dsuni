//! LP solver — simplex method with exact rational arithmetic.
//!
//! Provides a dense-tableau implementation of the two-phase simplex
//! algorithm for solving linear programs in standard form:
//!
//! ```text
//!   min   c^T x
//!   s.t.  A x = b
//!         x ≥ 0
//! ```

mod solver;
mod tableau;

pub use solver::*;
pub use tableau::*;

use crate::core::{Config, LpResult};

/// Solve a linear program given a `LinearSystem`.
///
/// This is the main public API for LP solving.
pub fn solve_lp(system: &crate::linear::LinearSystem, config: &Config) -> LpResult {
    let mut lp_solver = LpSolver::new(config.clone());
    lp_solver.load(system);
    lp_solver.solve()
}
