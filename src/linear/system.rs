use std::collections::HashMap;
use crate::core::Var;
use crate::linear::{Bound, Domain, LinearConstraint, Objective, Rational};

/// A complete linear system: variables, constraints, objective, and domains.
///
/// This is the main input container for LP / MILP / SMT-LRA solvers.
#[derive(Debug, Clone)]
pub struct LinearSystem {
    /// All variables that have been registered.
    vars: Vec<Var>,
    /// Constraints with optional names.
    constraints: Vec<LinearConstraint>,
    /// The objective function (default: feasibility).
    objective: Objective,
    /// Variable domains (type + bounds).
    domains: HashMap<Var, Domain>,
    /// Next variable ID to assign.
    next_var_id: usize,
}

impl LinearSystem {
    /// Create an empty linear system.
    pub fn new() -> Self {
        LinearSystem {
            vars: Vec::new(),
            constraints: Vec::new(),
            objective: Objective::None,
            domains: HashMap::new(),
            next_var_id: 0,
        }
    }

    // ── variable management ───────────────────────────────────────────

    /// Create a new real variable and return it.
    pub fn new_var(&mut self) -> Var {
        let v = Var::new(self.next_var_id);
        self.next_var_id += 1;
        self.vars.push(v);
        self.domains
            .insert(v, Domain::real(v));
        v
    }

    /// Create a new integer variable.
    pub fn new_int_var(&mut self) -> Var {
        let v = Var::new(self.next_var_id);
        self.next_var_id += 1;
        self.vars.push(v);
        self.domains
            .insert(v, Domain::integer(v));
        v
    }

    /// Register an existing variable with the system.
    pub fn add_var(&mut self, var: Var, domain: Domain) {
        if !self.vars.contains(&var) {
            self.vars.push(var);
        }
        self.domains.insert(var, domain);
        if var.id() >= self.next_var_id {
            self.next_var_id = var.id() + 1;
        }
    }

    /// All registered variables.
    pub fn vars(&self) -> &[Var] {
        &self.vars
    }

    /// Number of variables.
    pub fn num_vars(&self) -> usize {
        self.vars.len()
    }

    /// Domain for a specific variable.
    pub fn domain(&self, var: Var) -> Option<&Domain> {
        self.domains.get(&var)
    }

    /// Update the bound for a variable.
    pub fn set_bound(&mut self, var: Var, bound: Bound) {
        if let Some(domain) = self.domains.get_mut(&var) {
            domain.bound = bound;
        }
    }

    /// Set whether a variable is integer.
    pub fn set_integer(&mut self, var: Var, is_int: bool) {
        if let Some(domain) = self.domains.get_mut(&var) {
            domain.is_integer = is_int;
        }
    }

    /// Whether a variable is integer.
    pub fn is_integer(&self, var: Var) -> bool {
        self.domains
            .get(&var)
            .map(|d| d.is_integer)
            .unwrap_or(false)
    }

    // ── constraint management ─────────────────────────────────────────

    /// Add a constraint.
    pub fn add_constraint(&mut self, constraint: LinearConstraint) {
        self.constraints.push(constraint);
    }

    /// All constraints.
    pub fn constraints(&self) -> &[LinearConstraint] {
        &self.constraints
    }

    /// Number of constraints.
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    // ── objective ─────────────────────────────────────────────────────

    /// Set the objective function.
    pub fn set_objective(&mut self, objective: Objective) {
        self.objective = objective;
    }

    /// Get the objective function.
    pub fn objective(&self) -> &Objective {
        &self.objective
    }

    // ── helpers ───────────────────────────────────────────────────────

    /// Check whether a complete assignment satisfies all constraints and bounds.
    pub fn check_solution<F>(&self, mut get_value: F) -> bool
    where
        F: FnMut(Var) -> Rational,
    {
        // Check domain bounds
        for v in &self.vars {
            if let Some(domain) = self.domains.get(v) {
                let val = get_value(*v);
                if !domain.bound.contains(val) {
                    return false;
                }
            }
        }

        // Check all constraints
        self.constraints
            .iter()
            .all(|c| c.check(&mut get_value))
    }

    /// Clear all data.
    pub fn clear(&mut self) {
        self.vars.clear();
        self.constraints.clear();
        self.objective = Objective::None;
        self.domains.clear();
        self.next_var_id = 0;
    }
}

impl Default for LinearSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear::LinearExpr;

    #[test]
    fn test_empty_system() {
        let sys = LinearSystem::new();
        assert_eq!(sys.num_vars(), 0);
        assert_eq!(sys.num_constraints(), 0);
        assert!(sys.objective().is_feasibility());
    }

    #[test]
    fn test_new_var() {
        let mut sys = LinearSystem::new();
        let v = sys.new_var();
        assert_eq!(v, Var::new(0));
        assert_eq!(sys.num_vars(), 1);
        let d = sys.domain(v).unwrap();
        assert!(!d.is_integer);
    }

    #[test]
    fn test_new_int_var() {
        let mut sys = LinearSystem::new();
        let v = sys.new_int_var();
        assert!(sys.is_integer(v));
    }

    #[test]
    fn test_add_constraint() {
        let mut sys = LinearSystem::new();
        let c = LinearConstraint::le(LinearExpr::from(Rational::new(5, 1)));
        sys.add_constraint(c);
        assert_eq!(sys.num_constraints(), 1);
    }

    #[test]
    fn test_set_objective() {
        let mut sys = LinearSystem::new();
        let v = sys.new_var();
        sys.set_objective(Objective::Minimise(LinearExpr::from_var(v)));
        assert!(!sys.objective().is_feasibility());
    }

    #[test]
    fn test_set_bound() {
        let mut sys = LinearSystem::new();
        let v = sys.new_var();
        sys.set_bound(v, Bound::new(Some(Rational::ZERO), Some(Rational::new(10, 1))));
        let d = sys.domain(v).unwrap();
        assert_eq!(d.bound.lower, Some(Rational::ZERO));
        assert_eq!(d.bound.upper, Some(Rational::new(10, 1)));
    }
}
