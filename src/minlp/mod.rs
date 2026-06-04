//! MINLP solver — Mixed Integer NonLinear Programming.
//!
//! Uses interval bound propagation + McCormick relaxation + MILP.

mod interval;
mod solver;

pub use interval::*;
pub use solver::*;

use crate::core::{Config, LpResult};

/// Solve a MINLP problem.
pub fn solve_minlp(problem: &MinlpProblem, config: &Config) -> LpResult {
    let mut solver = MinlpSolver::new(config.clone());
    solver.solve(problem)
}

/// A MINLP problem: polynomial/bilinear constraints over bounded variables.
#[derive(Debug, Clone)]
pub struct MinlpProblem {
    pub vars: Vec<MinlpVar>,
    pub constraints: Vec<MinlpConstraint>,
}

#[derive(Debug, Clone)]
pub struct MinlpVar {
    pub name: String,
    pub lb: f64,
    pub ub: f64,
    pub is_integer: bool,
}

#[derive(Debug, Clone)]
pub struct MinlpConstraint {
    pub terms: Vec<MinlpTerm>,
    pub rhs: f64,
    pub is_eq: bool,
}

#[derive(Debug, Clone)]
pub enum MinlpTerm {
    Linear(f64, usize),       // coeff * x[i]
    Bilinear(f64, usize, usize), // coeff * x[i] * x[j]
    Square(f64, usize),       // coeff * x[i]²
}
