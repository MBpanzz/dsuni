use crate::cp::dom::IntDomain;
use crate::cp::propagator::{PropResult, Propagator};
use crate::cp::CpProblem;
use crate::core::{Assignment, Model, SolverResult, Value, Var};

/// The CP solver: propagation + backtracking search.
#[derive(Debug, Clone)]
pub struct CpSolver {
    /// Current domains for each variable.
    domains: Vec<IntDomain>,
    /// Propagators.
    propagators: Vec<Propagator>,
    /// Variable names.
    var_names: Vec<String>,
}

impl CpSolver {
    pub fn new() -> Self {
        CpSolver {
            domains: Vec::new(),
            propagators: Vec::new(),
            var_names: Vec::new(),
        }
    }

    /// Load a CP problem.
    pub fn load(&mut self, problem: &CpProblem) {
        self.domains = problem
            .var_domains
            .iter()
            .map(|(_, range)| IntDomain::new(*range.start(), *range.end()))
            .collect();
        self.var_names = problem.var_domains.iter().map(|(n, _)| n.clone()).collect();
        
        // Convert constraints to propagators
        self.propagators = problem.constraints.iter().map(|c| match c {
            crate::cp::CpConstraint::Eq(a, b) => Propagator::Eq(*a, *b),
            crate::cp::CpConstraint::Neq(a, b) => Propagator::Neq(*a, *b),
            crate::cp::CpConstraint::Lt(a, b) => Propagator::Lt(*a, *b),
            crate::cp::CpConstraint::Le(a, b, c) => Propagator::Le(*a, *b, *c),
            crate::cp::CpConstraint::Linear(terms, rhs, rel) => {
                Propagator::Linear(terms.clone(), *rhs, *rel)
            }
            crate::cp::CpConstraint::AllDifferent(vars) => {
                Propagator::AllDifferent(vars.clone())
            }
        }).collect();
    }

    /// Solve using propagation + backtracking.
    pub fn solve(&mut self) -> SolverResult {
        // Initial propagation
        if !self.propagate_all() {
            return SolverResult::Unsat;
        }

        // Check if all variables are fixed
        if self.all_fixed() {
            return SolverResult::Sat(self.build_model());
        }

        // Backtracking search
        let result = self.search(0);
        match result {
            Some(model) => SolverResult::Sat(model),
            None => SolverResult::Unsat,
        }
    }

    /// Backtracking search (DFS).
    fn search(&mut self, _depth: usize) -> Option<Model> {
        // Propagate to fixed point
        if !self.propagate_all() {
            return None;
        }

        // All fixed → solution found
        if self.all_fixed() {
            return Some(self.build_model());
        }

        // Select variable (smallest domain first — first-fail)
        let var = self.select_var()?;

        // Try values in ascending order
        let vals: Vec<i64> = (self.domains[var].min()..=self.domains[var].max())
            .filter(|&v| self.domains[var].contains(v))
            .collect();

        for val in vals {
            // Save state
            let old_domains = self.domains.clone();

            // Assign var = val
            self.domains[var] = IntDomain::single(val);

            // Recurse
            if let Some(model) = self.search(_depth + 1) {
                return Some(model);
            }

            // Restore
            self.domains = old_domains;
        }

        None
    }

    /// Select the variable with the smallest domain (first-fail principle).
    fn select_var(&self) -> Option<usize> {
        let mut best = None;
        let mut best_size = u64::MAX;
        for (i, d) in self.domains.iter().enumerate() {
            if !d.is_fixed() && d.size() < best_size && d.size() > 0 {
                best_size = d.size();
                best = Some(i);
            }
        }
        best
    }

    /// Run all propagators to fixed point.
    fn propagate_all(&mut self) -> bool {
        loop {
            // Take snapshot
            let snap: Vec<(i64, i64)> = self.domains
                .iter().map(|d| (d.min(), d.max())).collect();
            
            for p in &self.propagators {
                if let PropResult::Failure = p.propagate(&mut self.domains) {
                    return false;
                }
            }
            
            // Check if any domain changed
            let any_changed = self.domains.iter().zip(snap.iter()).any(|(d, &(mn, mx))| {
                d.min() != mn || d.max() != mx
            });
            
            if !any_changed {
                return true; // Fixed point reached
            }
        }
    }

    /// Check if all variables are fixed.
    fn all_fixed(&self) -> bool {
        self.domains.iter().all(|d| d.is_fixed())
    }

    /// Build a model from fixed domains.
    fn build_model(&self) -> Model {
        let mut assignment = Assignment::new();
        for (i, d) in self.domains.iter().enumerate() {
            if let Some(val) = d.value() {
                assignment.set(Var::new(i), Value::Int(val));
            }
        }
        Model::new(assignment)
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cp::{CpConstraint, CpProblem};

    #[test]
    fn test_simple_eq() {
        let problem = CpProblem {
            var_domains: vec![
                ("x".into(), 0..=10),
                ("y".into(), 0..=10),
            ],
            constraints: vec![
                CpConstraint::Eq(0, 1),
            ],
        };
        let mut solver = CpSolver::new();
        solver.load(&problem);
        let result = solver.solve();
        assert!(result.is_sat(), "Expected SAT, got {:?}", result);
    }

    #[test]
    fn test_unsat() {
        let problem = CpProblem {
            var_domains: vec![
                ("x".into(), 0..=5),
            ],
            constraints: vec![
                CpConstraint::Lt(0, 0), // x < x → impossible
            ],
        };
        let mut solver = CpSolver::new();
        solver.load(&problem);
        let result = solver.solve();
        assert!(result.is_unsat(), "Expected UNSAT, got {:?}", result);
    }
}
