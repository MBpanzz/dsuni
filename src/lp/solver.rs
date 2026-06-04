use crate::core::{Config, LpResult, Model, UnknownReason};
#[allow(unused_imports)]
use crate::linear::{
    CmpOp, LinearConstraint, LinearExpr, LinearSystem, Objective, Rational, Var,
};
use crate::lp::tableau::Tableau;

/// The LP solver using the two-phase simplex method with exact rational arithmetic.
#[derive(Debug, Clone)]
pub struct LpSolver {
    #[allow(dead_code)]
    config: Config,
    /// The linear system to solve.
    system: Option<LinearSystem>,
    /// The simplex tableau (built during solve).
    #[allow(dead_code)]
    tableau: Option<Tableau>,
    /// For each tableau column (structural + slack + artificial), maps back to
    /// `(Var, is_original)` where `is_original` distinguishes original from slack/artificial.
    col_map: Vec<(Var, bool)>,
}

impl LpSolver {
    /// Create a new LP solver.
    pub fn new(config: Config) -> Self {
        LpSolver {
            config,
            system: None,
            tableau: None,
            col_map: Vec::new(),
        }
    }

    /// Load a linear system for solving.
    pub fn load(&mut self, system: &LinearSystem) {
        self.system = Some(system.clone());
    }

    /// Solve the loaded LP.
    pub fn solve(&mut self) -> LpResult {
        let system = self
            .system
            .as_ref()
            .ok_or_else(|| LpResult::Unknown(UnknownReason::InternalReason(
                "no system loaded".into(),
            )));

        // Clone to avoid borrow issues
        let system = match system {
            Ok(s) => s.clone(),
            Err(e) => return e,
        };

        // 1. Build the initial tableau (standard form conversion)
        let (mut tableau, col_map, artificial_cols) = build_initial_tableau(&system);
        self.col_map = col_map;

        // Set objective coefficients in the tableau (row 0).
        // Store `-c_j` so that after elimination the reduced costs are correct.
        if let Some(obj_expr) = system.objective().expr() {
            for term in &obj_expr.terms {
                let coeff = match system.objective() {
                    // For minimization: start from z - c^T x = 0, so store -c_j
                    Objective::Minimise(_) => -term.coefficient.clone(),
                    // For maximization: max c^T x = min -c^T x, store -(-c_j) = c_j
                    Objective::Maximise(_) => term.coefficient.clone(),
                    Objective::None => Rational::ZERO,
                };
                tableau.set_obj_coeff(term.var.id(), coeff);
            }
            // Constant part: store -const in RHS
            let const_val = match system.objective() {
                Objective::Minimise(e) | Objective::Maximise(e) => {
                    -e.constant.clone()
                }
                Objective::None => Rational::ZERO,
            };
            if !const_val.is_zero() {
                *tableau.get_mut(0, tableau.num_cols - 1) = const_val;
            }
        }

        // Eliminate basic variables from the objective row.
        // For each constraint row, if the basic variable has a non-zero coefficient
        // in row 0, add the constraint row to eliminate it.
        for row in 1..tableau.num_rows() {
            if let Some(basic_col) = tableau.basic[row] {
                let coeff = tableau.get(0, basic_col).clone();
                if !coeff.is_zero() {
                    for col in 0..tableau.num_cols {
                        let val = tableau.get_mut(0, col).clone();
                        *tableau.get_mut(0, col) =
                            val - coeff.clone() * tableau.get(row, col).clone();
                    }
                }
            }
        }

        // (Debug print can be added here if needed)

        // 2. Phase I: find a feasible basis
        let feasible = phase_i(&mut tableau, &artificial_cols);
        if !feasible {
            return LpResult::Infeasible;
        }

        // 3. Phase II: optimize
        let result = phase_ii(&mut tableau);
        match result {
            PhaseResult::Optimal => {
                let model = extract_model(&tableau, &self.col_map, &system);
                let mut obj_val = tableau.objective_value();
                // For maximize problems, negate the result back
                if matches!(system.objective(), Objective::Maximise(_)) {
                    obj_val = -obj_val;
                }
                LpResult::Optimal(model, obj_val.to_f64())
            }
            PhaseResult::Unbounded => LpResult::Unbounded,
            PhaseResult::Infeasible => LpResult::Infeasible,
        }
    }
}

// ── Standard-form conversion ──────────────────────────────────────────

/// Build the initial simplex tableau for a linear system.
///
/// Converts inequality constraints to equalities using slack/surplus/artificial variables.
///
/// Standard form conversion:
/// - `a^T x ≤ b` → `a^T x + s = b`, s ≥ 0 (slack)
/// - `a^T x ≥ b` → `a^T x - s + a = b`, s ≥ 0 (surplus), a ≥ 0 (artificial)
/// - `a^T x = b` → `a^T x + a = b`, a ≥ 0 (artificial)
///
/// All RHS values are made non-negative by row negation.
/// Returns `(tableau, col_map, artificial_cols)` where `artificial_cols`
/// contains the column indices of artificial variables used in Phase I.
fn build_initial_tableau(system: &LinearSystem) -> (Tableau, Vec<(Var, bool)>, Vec<usize>) {
    let num_orig_vars = system.num_vars();
    let orig_constraints = system.constraints().to_vec();

    // Count total slack/surplus/artificial columns needed.
    let mut total_slack = 0usize;

    for c in &orig_constraints {
        match c.op {
            CmpOp::Le => total_slack += 1,
            CmpOp::Ge => total_slack += 2, // surplus + artificial
            CmpOp::Eq => total_slack += 1, // artificial
        }
    }

    let num_constraints = orig_constraints.len();
    let mut tableau = Tableau::new(num_orig_vars + total_slack, num_constraints);

    // Column mapping: original vars first, then slack/surplus/artificial.
    let mut col_map: Vec<(Var, bool)> = system
        .vars()
        .iter()
        .map(|v| (*v, true))
        .collect();

    // Track which columns are artificial for Phase I.
    let mut artificial_cols: Vec<usize> = Vec::new();

    // Fill constraint rows.
    // We first collect all coefficients, then if RHS is negative we
    // negate the entire row.
    let mut slack_col = num_orig_vars;

    for (i, c) in orig_constraints.iter().enumerate() {
        // Determine RHS and whether we need to negate.
        // Standard form: a^T x + const [≤|=|≥] 0
        // → a^T x [≤|=|≥] -const
        let mut rhs = -c.expr.constant.clone();
        let mut row_negated = false;

        if rhs.is_negative() {
            rhs = -rhs;
            row_negated = true;
        }
        tableau.set_rhs(i, rhs);

        // Coefficients for original variables.
        for term in &c.expr.terms {
            let coeff = if row_negated {
                -term.coefficient.clone()
            } else {
                term.coefficient.clone()
            };
            tableau.set_coeff(i, term.var.id(), coeff);
        }

        // Slack/surplus/artificial columns.
        match c.op {
            CmpOp::Le => {
                // Slack: +1 (or -1 if row negated)
                let coeff = if row_negated {
                    Rational::NEG_ONE
                } else {
                    Rational::ONE
                };
                tableau.set_coeff(i, slack_col, coeff);
                col_map.push((Var::new(1000 + i), false));
                // Slack is the basic variable for this row (if coeff is +1).
                if !row_negated {
                    tableau.set_basic(i + 1, slack_col);
                }
                slack_col += 1;
            }
            CmpOp::Ge => {
                // Surplus: -1 (or +1 if row negated)
                let sc = if row_negated { Rational::ONE } else { Rational::NEG_ONE };
                tableau.set_coeff(i, slack_col, sc);
                col_map.push((Var::new(1000 + i), false));
                slack_col += 1;
                // Artificial: +1 (or -1 if row negated)
                let ac = if row_negated { Rational::NEG_ONE } else { Rational::ONE };
                tableau.set_coeff(i, slack_col, ac);
                col_map.push((Var::new(2000 + i), false));
                artificial_cols.push(slack_col);
                // Artificial is the basic variable.
                tableau.set_basic(i + 1, slack_col);
                slack_col += 1;
            }
            CmpOp::Eq => {
                // Artificial: +1 (or -1 if row negated)
                let ac = if row_negated { Rational::NEG_ONE } else { Rational::ONE };
                tableau.set_coeff(i, slack_col, ac);
                col_map.push((Var::new(2000 + i), false));
                artificial_cols.push(slack_col);
                // Artificial is the basic variable.
                tableau.set_basic(i + 1, slack_col);
                slack_col += 1;
            }
        }
    }

    (tableau, col_map, artificial_cols)
}

// ── Phase I ───────────────────────────────────────────────────────────

/// Result of Phase II or a simplex iteration.
#[derive(Debug, Clone, PartialEq)]
enum PhaseResult {
    Optimal,
    Unbounded,
    #[allow(dead_code)]
    Infeasible,
}

/// Phase I: find a feasible basis by minimising artificial variables.
///
/// Returns `true` if a feasible basis was found.
fn phase_i(tableau: &mut Tableau, artificial_cols: &[usize]) -> bool {
    // All RHS should already be non-negative from build_initial_tableau.
    // If not, negate the row.
    for row in 1..tableau.num_rows() {
        if tableau.rhs(row).is_negative() {
            for col in 0..tableau.num_cols {
                let val = tableau.get_mut(row, col);
                *val = -val.clone();
            }
        }
    }

    if artificial_cols.is_empty() {
        return true; // Already feasible
    }

    // Save original objective row before overwriting.
    let orig_obj: Vec<Rational> = (0..tableau.num_cols)
        .map(|col| tableau.get(0, col).clone())
        .collect();

    // Clear the objective row entirely for the auxiliary problem.
    for col in 0..tableau.num_cols - 1 {
        *tableau.get_mut(0, col) = Rational::ZERO;
    }
    *tableau.get_mut(0, tableau.num_cols - 1) = Rational::ZERO;

    // Set auxiliary objective: minimize sum of artificials.
    // In z - c^T x = 0 form, c_j = 1 for each artificial, so set -1.
    for &col in artificial_cols {
        *tableau.get_mut(0, col) = Rational::NEG_ONE;
    }

    // Eliminate basic artificial variables from the objective row
    // so their reduced costs become 0.
    for row in 1..tableau.num_rows() {
        if let Some(basic_col) = tableau.basic[row] {
            if artificial_cols.contains(&basic_col) {
                let pivot = tableau.get(0, basic_col).clone();
                if !pivot.is_zero() {
                    for col in 0..tableau.num_cols {
                        let val = tableau.get(0, col).clone();
                        *tableau.get_mut(0, col) =
                            val - pivot.clone() * tableau.get(row, col).clone();
                    }
                }
            }
        }
    }

    // Now run simplex on the auxiliary problem.
    loop {
        let entering = tableau.find_entering();
        if entering.is_none() {
            break; // optimal for auxiliary
        }
        let entering = entering.unwrap();

        let leaving = tableau.find_leaving(entering);
        if leaving.is_none() {
            // Unbounded auxiliary → shouldn't happen, but treat as infeasible
            return false;
        }
        let (leaving_row, _) = leaving.unwrap();
        tableau.pivot(leaving_row, entering);
    }

    // Check auxiliary objective value
    let aux_obj = tableau.objective_value();
    if !aux_obj.is_zero() {
        return false; // Infeasible
    }

    // Restore original objective
    for col in 0..tableau.num_cols {
        *tableau.get_mut(0, col) = orig_obj[col].clone();
    }

    // Re-express original objective in terms of non-basic variables
    // by eliminating basic variables from row 0
    for row in 1..tableau.num_rows() {
        if let Some(basic_col) = tableau.basic[row] {
            let pivot_val = tableau.get(row, basic_col).clone();
            if pivot_val.is_zero() {
                continue;
            }
            let factor = tableau.get(0, basic_col).clone();
            if !factor.is_zero() {
                let ratio = factor / pivot_val;
                for col in 0..tableau.num_cols {
                    let val = tableau.get(0, col).clone();
                    *tableau.get_mut(0, col) =
                        val - ratio.clone() * tableau.get(row, col).clone();
                }
            }
        }
    }

    true
}

// ── Phase II ──────────────────────────────────────────────────────────

/// Phase II: optimise the original objective.
fn phase_ii(tableau: &mut Tableau) -> PhaseResult {
    loop {
        let entering = tableau.find_entering();
        if entering.is_none() {
            return PhaseResult::Optimal;
        }
        let entering = entering.unwrap();

        let leaving = tableau.find_leaving(entering);
        if leaving.is_none() {
            return PhaseResult::Unbounded;
        }
        let (leaving_row, _) = leaving.unwrap();
        tableau.pivot(leaving_row, entering);
    }
}

// ── Model extraction ──────────────────────────────────────────────────

/// Extract a model from the final tableau.
fn extract_model(
    tableau: &Tableau,
    col_map: &[(Var, bool)],
    _system: &LinearSystem,
) -> Model {
    use crate::core::{Assignment, Value};

    let mut assignment = Assignment::new();
    for (col, (var, is_original)) in col_map.iter().enumerate() {
        if *is_original {
            let val = tableau.variable_value(col);
            let value = if val.den() == 1 {
                Value::Int(val.num())
            } else {
                Value::Real(val.to_f64())
            };
            assignment.set(*var, value);
        }
    }
    Model::new(assignment)
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear::LinearSystem;

    /// Simple LP: min -x0 s.t. x0 <= 5, x0 >= 0 → optimal = -5 at x0=5
    #[test]
    fn test_simple_lp() {
        let mut sys = LinearSystem::new();
        let x = sys.new_var();
        sys.add_constraint(LinearConstraint::le(
            LinearExpr::from_var(x) - LinearExpr::from(5i64),
        ));
        sys.set_objective(Objective::Minimise(
            LinearExpr::from_var(x) * Rational::NEG_ONE,
        ));

        let mut solver = LpSolver::new(Config::default());
        solver.load(&sys);
        let result = solver.solve();
        match result {
            LpResult::Optimal(model, obj) => {
                assert!((obj - (-5.0)).abs() < 1e-10, "obj={}", obj);
                let val = model.assignments.get(&x);
                assert!(val.is_some());
            }
            other => panic!("Expected optimal, got {:?}", other),
        }
    }

    /// Infeasible: x0 <= 0, x0 >= 5
    #[test]
    fn test_infeasible() {
        let mut sys = LinearSystem::new();
        let x = sys.new_var();
        sys.add_constraint(LinearConstraint::le(
            LinearExpr::from_var(x),
        ));
        sys.add_constraint(LinearConstraint::ge(
            LinearExpr::from_var(x) - LinearExpr::from(5i64),
        ));

        let mut solver = LpSolver::new(Config::default());
        solver.load(&sys);
        let result = solver.solve();
        assert_eq!(result, LpResult::Infeasible);
    }
}
