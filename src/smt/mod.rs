//! SMT solver — CDCL(T) framework with theory plugins.
//!
//! Architecture:
//!   SMT formula  →  Boolean abstraction  →  SAT solver
//!                                           ↓
//!                               Theory solver (UF / LRA / ...)

mod bool_abstraction;
mod cdclt;
mod lra;
pub mod theory;
mod uf;

pub use bool_abstraction::*;
pub use cdclt::*;
pub use lra::*;
pub use theory::{TheoryAtom, TheoryCheckResult, TheorySolver};
pub use uf::*;

use crate::core::{Config, SolverResult};

/// Solve an SMT problem from a parsed formula.
pub fn solve_smt(formula: &Formula, _config: &Config) -> SolverResult {
    let mut solver = CDCLT::new();
    // Add UF theory solver by default
    solver.add_theory(Box::new(crate::smt::uf::UF::new()));
    solver.load(formula);
    solver.solve()
}

/// An SMT formula: a set of assertions (clauses of theory literals).
#[derive(Debug, Clone, Default)]
pub struct Formula {
    pub atoms: Vec<Box<dyn crate::smt::theory::TheoryAtom>>,
    pub clauses: Vec<Vec<TheoryLit>>,
}

/// A theory literal (atom ± sign).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TheoryLit {
    pub atom_id: usize,
    pub sign: bool,
}

impl TheoryLit {
    pub fn new(atom_id: usize, sign: bool) -> Self {
        TheoryLit { atom_id, sign }
    }
    pub fn negate(&self) -> Self {
        TheoryLit { atom_id: self.atom_id, sign: !self.sign }
    }
}
