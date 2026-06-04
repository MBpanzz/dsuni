//! SAT solver module — CDCL with two-watched literals, VSIDS, and conflict-driven learning.
//!
//! Public entry point: [`solve_sat`].

mod clause;
mod heuristics;
mod literal;
mod solver;
mod trail;
mod watch;

pub use clause::*;
pub use heuristics::*;
pub use literal::*;
pub use solver::*;
pub use trail::*;
pub use watch::*;

use crate::core::{Config, SolverResult};

/// Solve a CNF formula given as a list of clauses.
///
/// Each clause is a `Vec<i32>` where the integer's absolute value is the
/// 1-indexed variable number and the sign indicates polarity (positive = true).
pub fn solve_sat(clauses: &[Vec<i32>], config: &Config) -> SolverResult {
    let mut solver = Solver::new(config.clone());
    for raw_clause in clauses {
        let lits: Vec<Lit> = raw_clause
            .iter()
            .map(|&i| Lit::from_dimacs(i))
            .collect();
        solver.add_clause(&lits);
    }
    solver.solve()
}
