//! QF_LRA theory solver — linear real arithmetic.
//!
//! Reuses the LP solver to check consistency of linear constraints.

use crate::linear::{CmpOp, LinearConstraint, LinearExpr, LinearSystem, Var};
use crate::lp::solve_lp;
use crate::core::{Config, LpResult};
use crate::smt::{TheoryAtom, TheoryCheckResult, TheorySolver};
use crate::smt::TheoryLit;

/// A linear arithmetic atom: `expr ≤ 0`, `expr = 0`, or `expr ≥ 0`.
#[derive(Debug, Clone)]
pub struct LraAtom {
    pub id: usize,
    pub expr: LinearExpr,
    pub op: CmpOp,
}

impl TheoryAtom for LraAtom {
    fn id(&self) -> usize {
        self.id
    }
    fn label(&self) -> String {
        format!("{} {} 0", self.expr, match self.op {
            CmpOp::Le => "<=",
            CmpOp::Ge => ">=",
            CmpOp::Eq => "=",
        })
    }
    fn clone_box(&self) -> Box<dyn TheoryAtom> {
        Box::new(self.clone())
    }
}

/// LRA theory solver.
///
/// Collects linear constraints and uses the LP solver to check consistency.
#[derive(Debug, Clone)]
pub struct LRA {
    /// All linear constraints indexed by atom ID.
    atoms: Vec<LraAtom>,
    /// Currently asserted literals (atom_id, sign).
    asserted: Vec<(usize, bool)>,
    /// Linear system built from asserted literals.
    system: Option<LinearSystem>,
    /// Stack for push/pop.
    stack: Vec<usize>,
    /// Next variable ID.
    next_var: usize,
}

impl LRA {
    pub fn new() -> Self {
        LRA {
            atoms: Vec::new(),
            asserted: Vec::new(),
            system: None,
            stack: Vec::new(),
            next_var: 0,
        }
    }

    /// Register a new LRA atom.
    pub fn register_atom(&mut self, atom: LraAtom) -> usize {
        let id = self.atoms.len();
        self.atoms.push(atom);
        id
    }

    /// Build the current linear system from asserted literals.
    fn build_system(&self) -> LinearSystem {
        let mut sys = LinearSystem::new();
        // Create variables for each term variable
        let mut var_map: std::collections::HashMap<usize, Var> = std::collections::HashMap::new();
        for &(atom_id, _sign) in &self.asserted {
            let atom = &self.atoms[atom_id];
            for term in &atom.expr.terms {
                if !var_map.contains_key(&term.var.id()) {
                    let v = sys.new_var();
                    var_map.insert(term.var.id(), v);
                }
            }
        }

        for &(atom_id, sign) in &self.asserted {
            let atom = &self.atoms[atom_id];
            // Map the expression variables
            let op = if sign { atom.op } else { atom.op.flip() };
            sys.add_constraint(LinearConstraint::new(atom.expr.clone(), op));
        }

        sys
    }
}

impl TheorySolver for LRA {
    fn push(&mut self) {
        self.stack.push(self.asserted.len());
    }

    fn pop(&mut self) {
        if let Some(len) = self.stack.pop() {
            self.asserted.truncate(len);
        }
    }

    fn assert(&mut self, lit: TheoryLit) -> bool {
        self.asserted.push((lit.atom_id, lit.sign));
        true
    }

    fn check(&self) -> TheoryCheckResult {
        if self.asserted.is_empty() {
            return TheoryCheckResult::Consistent;
        }
        let sys = self.build_system();
        let result = solve_lp(&sys, &Config::default());
        match result {
            LpResult::Optimal(_, _) | LpResult::Feasible(_) => {
                TheoryCheckResult::Consistent
            }
            LpResult::Infeasible => TheoryCheckResult::Conflict,
            _ => TheoryCheckResult::Unknown,
        }
    }

    fn propagate(&self) -> Vec<TheoryLit> {
        vec![] // Basic LRA: no early propagation
    }

    fn explain_conflict(&self) -> Vec<TheoryLit> {
        // Return all asserted literals as the conflict explanation
        self.asserted.iter().map(|&(id, sign)| TheoryLit::new(id, sign)).collect()
    }

    fn explain_propagation(&self, _lit: TheoryLit) -> Vec<TheoryLit> {
        vec![]
    }

    fn set_atoms(&mut self, _atoms: &[Box<dyn TheoryAtom>]) {}

    fn reset(&mut self) {
        self.atoms.clear();
        self.asserted.clear();
        self.system = None;
        self.stack.clear();
        self.next_var = 0;
    }
}
