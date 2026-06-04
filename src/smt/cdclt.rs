//! CDCL(T) — the SMT core loop integrating SAT with theory solvers.

use crate::core::{Config, Model, SolverResult};
use crate::sat::Lit;
use crate::smt::{Formula, TheoryAtom, TheoryCheckResult, TheoryLit, TheorySolver};

/// A clause in the SMT formula: a disjunction of theory literals.
#[derive(Debug, Clone)]
pub struct TClause {
    pub lits: Vec<TheoryLit>,
}

/// The CDCL(T) SMT solver.
#[derive(Debug)]
pub struct CDCLT {
    /// Theory solvers.
    theory_solvers: Vec<Box<dyn TheorySolver>>,
    /// Clauses of the original formula.
    clauses: Vec<TClause>,
    /// Number of atoms.
    num_atoms: usize,
}

impl CDCLT {
    pub fn new() -> Self {
        CDCLT {
            theory_solvers: Vec::new(),
            clauses: Vec::new(),
            num_atoms: 0,
        }
    }

    /// Add a theory solver.
    pub fn add_theory(&mut self, solver: Box<dyn TheorySolver>) {
        self.theory_solvers.push(solver);
    }

    /// Load an SMT formula.
    pub fn load(&mut self, formula: &Formula) {
        self.clauses = formula
            .clauses
            .iter()
            .map(|lits| TClause { lits: lits.clone() })
            .collect();
        self.num_atoms = formula.atoms.len();

        // Pass atoms to theory solvers
        for ts in &mut self.theory_solvers {
            ts.set_atoms(&formula.atoms);
        }
    }

    /// Solve the loaded SMT problem using lazy CDCL(T).
    ///
    /// Main loop: SAT → theory check → if conflict, add theory lemma
    /// and re-solve SAT. Repeats until SAT+theory model found or SAT UNSAT.
    pub fn solve(&mut self) -> SolverResult {
        if self.num_atoms == 0 && self.clauses.is_empty() {
            return SolverResult::Sat(Model::new(crate::core::Assignment::new()));
        }

        let mut sat_solver = crate::sat::Solver::new(Config::default());
        for tc in &self.clauses {
            let sat_lits: Vec<Lit> = tc
                .lits
                .iter()
                .map(|tl| Lit::new(tl.atom_id, tl.sign))
                .collect();
            sat_solver.add_clause(&sat_lits);
        }

        // CDCL(T) loop: SAT → theory → conflict clause → SAT again
        let max_iterations = 100;
        for _iter in 0..max_iterations {
            let sat_result = sat_solver.solve();

            match sat_result {
                SolverResult::Sat(model) => {
                    let assignment: Vec<Option<bool>> = (0..self.num_atoms)
                        .map(|i| {
                            model.assignments
                                .get(&crate::core::Var::new(i))
                                .and_then(|v| match v {
                                    crate::core::Value::Bool(b) => Some(*b),
                                    _ => None,
                                })
                        })
                        .collect();

                    // Check theory consistency
                    for ts in &mut self.theory_solvers {
                        for (atom_id, val) in assignment.iter().enumerate() {
                            if let Some(v) = val {
                                ts.assert(TheoryLit::new(atom_id, *v));
                            }
                        }
                        let result = ts.check();
                        match result {
                            TheoryCheckResult::Conflict => {
                                // Add conflict clause (negation of assignment)
                                let conflict_clause: Vec<Lit> = assignment.iter().enumerate()
                                    .filter_map(|(i, v)| v.map(|b| Lit::new(i, !b)))
                                    .collect();
                                sat_solver.add_clause(&conflict_clause);
                                // Reset theory solvers for next iteration
                                ts.reset();
                                break; // go back to SAT solving
                            }
                            TheoryCheckResult::Consistent | TheoryCheckResult::Unknown => {}
                        }
                    }

                    // If we got here, all theory solvers are consistent
                    if self.theory_solvers.iter().all(|ts| {
                        ts.check() == TheoryCheckResult::Consistent
                    }) {
                        return SolverResult::Sat(model);
                    }
                    // Otherwise, loop continues (conflict was added)
                }
                SolverResult::Unsat => return SolverResult::Unsat,
                other => return other,
            }
        }

        SolverResult::Unsat // exceeded max iterations
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smt::Formula;

    #[test]
    fn test_empty_formula() {
        let mut solver = CDCLT::new();
        let formula = Formula::default();
        solver.load(&formula);
        let result = solver.solve();
        assert!(result.is_sat());
    }

    #[test]
    fn test_simple_unsat() {
        // Two opposing unit clauses
        let mut formula = Formula::default();
        formula.clauses.push(vec![TheoryLit::new(0, true)]);
        formula.clauses.push(vec![TheoryLit::new(0, false)]);
        // Need to add a placeholder atom so num_atoms > 0
        formula.atoms.push(Box::new(DummyAtom { id: 0 }));

        let mut solver = CDCLT::new();
        solver.load(&formula);
        let result = solver.solve();
        assert!(result.is_unsat());
    }
}

/// Dummy atom for testing.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DummyAtom { id: usize }

impl TheoryAtom for DummyAtom {
    fn id(&self) -> usize { self.id }
    fn label(&self) -> String { format!("a{}", self.id) }
    fn clone_box(&self) -> Box<dyn TheoryAtom> { Box::new(self.clone()) }
}
