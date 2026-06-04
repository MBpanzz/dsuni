use crate::core::{Config, LpResult};
use crate::linear::{Bound, LinearConstraint, LinearExpr, LinearSystem, Objective, Rational, Var};
use crate::milp::solve_milp;
use crate::minlp::interval::{self, Interval};
use crate::minlp::{MinlpProblem, MinlpTerm};

/// MINLP solver using interval propagation + McCormick relaxation.
#[derive(Debug, Clone)]
pub struct MinlpSolver {
    config: Config,
}

impl MinlpSolver {
    pub fn new(config: Config) -> Self {
        MinlpSolver { config }
    }

    pub fn solve(&mut self, problem: &MinlpProblem) -> LpResult {
        if problem.vars.is_empty() {
            return LpResult::Optimal(
                crate::core::Model::new(crate::core::Assignment::new()),
                0.0,
            );
        }

        // 1. Interval bound propagation
        let mut intervals: Vec<Interval> = problem
            .vars
            .iter()
            .map(|v| Interval::new(v.lb, v.ub))
            .collect();

        let mut changed = true;
        while changed {
            changed = false;
            for c in &problem.constraints {
                let constraint_iv = eval_interval(&c.terms, &intervals);
                if constraint_iv.is_empty() {
                    return LpResult::Infeasible;
                }
                // Constraint: expr ≤ 0 or expr = 0
                // Constraint: expr ≤ rhs or expr = rhs
                // Check if the constraint's interval is compatible with the target.
                if c.is_eq && !constraint_iv.contains(c.rhs) {
                    return LpResult::Infeasible;
                }
                if !c.is_eq && constraint_iv.lb > c.rhs {
                    return LpResult::Infeasible;
                }
                // Propagate bounds back through each term
                for term in &c.terms {
                    match term {
                        MinlpTerm::Linear(coeff, var) => {
                            if *coeff == 0.0 { continue; }
                            // From Σ(coeff_i * x_i) ≤ rhs:
                            // coeff * x_var ≤ rhs - Σ_other(coeff_j * x_j)
                            let other_sum_iv = eval_interval_except(&c.terms, &intervals, *var);
                            let rhs_iv = Interval::new(c.rhs, c.rhs);
                            let max_for_var = sub(&rhs_iv, &other_sum_iv);
                            let var_iv = if *coeff > 0.0 {
                                Interval::new(
                                    max_for_var.lb / *coeff,
                                    max_for_var.ub / *coeff,
                                )
                            } else {
                                Interval::new(
                                    max_for_var.ub / *coeff,
                                    max_for_var.lb / *coeff,
                                )
                            };
                            let new_iv = intervals[*var].intersect(&var_iv);
                            if new_iv.is_empty() { return LpResult::Infeasible; }
                            if (new_iv.lb - intervals[*var].lb).abs() > 1e-10
                                || (new_iv.ub - intervals[*var].ub).abs() > 1e-10
                            {
                                intervals[*var] = new_iv;
                                changed = true;
                            }
                        }
                        _ => {} // Nonlinear terms: harder to back-propagate
                    }
                }
            }
        }

        // 2. Build MILP relaxation with McCormick envelopes for bilinear terms
        let mut milp_sys = LinearSystem::new();

        // Create variables
        let mut milp_vars: Vec<Var> = Vec::new();
        for v in &problem.vars {
            let var = if v.is_integer {
                milp_sys.new_int_var()
            } else {
                milp_sys.new_var()
            };
            milp_vars.push(var);
            milp_sys.set_bound(
                var,
                Bound::new(
                    Some(Rational::from_f64(v.lb)),
                    Some(Rational::from_f64(v.ub)),
                ),
            );
        }

        // Track auxiliary variables for bilinear terms
        // (w = x*y with McCormick constraints)
        let mut aux_vars: Vec<(Var, Interval)> = Vec::new();
        let mut aux_terms: Vec<(usize, usize, usize)> = Vec::new();

        // Process constraints
        for c in &problem.constraints {
            let mut linear_expr = LinearExpr::ZERO;

            for term in &c.terms {
                match term {
                    MinlpTerm::Linear(coeff, var_idx) => {
                        let mv = milp_vars[*var_idx];
                        let term_expr = LinearExpr::from_var(mv) * Rational::from_f64(*coeff);
                        linear_expr = linear_expr + term_expr;
                    }
                    MinlpTerm::Bilinear(coeff, i, j) => {
                        // Create McCormick relaxation for x_i * x_j
                        let xi_iv = intervals[*i];
                        let xj_iv = intervals[*j];
                        let aux_idx = aux_vars.len();
                        let w = milp_sys.new_var();
                        milp_sys.set_bound(
                            w,
                            Bound::new(
                                Some(Rational::from_f64(xi_iv.lb * xj_iv.lb)),
                                Some(Rational::from_f64(xi_iv.ub * xj_iv.ub)),
                            ),
                        );
                        // McCormick envelopes:
                        // w ≥ xi_lb * xj + xj_lb * xi - xi_lb * xj_lb
                        // w ≥ xi_ub * xj + xj_ub * xi - xi_ub * xj_ub
                        // w ≤ xi_lb * xj + xj_ub * xi - xi_lb * xj_ub
                        // w ≤ xi_ub * xj + xj_lb * xi - xi_ub * xj_lb

                        let xi = milp_vars[*i];
                        let xj = milp_vars[*j];
                        let (xl, xu) = (xi_iv.lb, xi_iv.ub);
                        let (yl, yu) = (xj_iv.lb, xj_iv.ub);

                        // Lower bound 1: w ≥ xl*y + yl*x - xl*yl
                        let mut lb1 = LinearExpr::from_var(xj) * Rational::from_f64(xl);
                        lb1 = lb1 + LinearExpr::from_var(xi) * Rational::from_f64(yl);
                        lb1 = lb1 - LinearExpr::constant(Rational::from_f64(xl * yl));
                        milp_sys.add_constraint(LinearConstraint::ge(
                            LinearExpr::from_var(w) - lb1,
                        ));

                        // Lower bound 2: w ≥ xu*y + yu*x - xu*yu
                        let mut lb2 = LinearExpr::from_var(xj) * Rational::from_f64(xu);
                        lb2 = lb2 + LinearExpr::from_var(xi) * Rational::from_f64(yu);
                        lb2 = lb2 - LinearExpr::constant(Rational::from_f64(xu * yu));
                        milp_sys.add_constraint(LinearConstraint::ge(
                            LinearExpr::from_var(w) - lb2,
                        ));

                        // Upper bound 1: w ≤ xl*y + yu*x - xl*yu
                        let mut ub1 = LinearExpr::from_var(xj) * Rational::from_f64(xl);
                        ub1 = ub1 + LinearExpr::from_var(xi) * Rational::from_f64(yu);
                        ub1 = ub1 - LinearExpr::constant(Rational::from_f64(xl * yu));
                        milp_sys.add_constraint(LinearConstraint::le(
                            LinearExpr::from_var(w) - ub1,
                        ));

                        // Upper bound 2: w ≤ xu*y + yl*x - xu*yl
                        let mut ub2 = LinearExpr::from_var(xj) * Rational::from_f64(xu);
                        ub2 = ub2 + LinearExpr::from_var(xi) * Rational::from_f64(yl);
                        ub2 = ub2 - LinearExpr::constant(Rational::from_f64(xu * yl));
                        milp_sys.add_constraint(LinearConstraint::le(
                            LinearExpr::from_var(w) - ub2,
                        ));

                        aux_vars.push((w, mul(&xi_iv, &xj_iv)));
                        aux_terms.push((aux_idx, *i, *j));
                        let term_expr = LinearExpr::from_var(w) * Rational::from_f64(*coeff);
                        linear_expr = linear_expr + term_expr;
                    }
                    MinlpTerm::Square(coeff, var_idx) => {
                        // x²: use the bilinear with x*x
                        let xi_iv = intervals[*var_idx];
                        let _aux_idx = aux_vars.len();
                        let w = milp_sys.new_var();
                        milp_sys.set_bound(
                            w,
                            Bound::new(
                                Some(Rational::ZERO),
                                Some(Rational::from_f64(xi_iv.ub.abs().max(xi_iv.lb.abs()).powi(2))),
                            ),
                        );
                        // For convex relaxation of x²:
                        // w ≥ 2*xl*x - xl² (tangent at xl)
                        // w ≥ 2*xu*x - xu² (tangent at xu)
                        // w ≤ (xl+xu)*x - xl*xu (secant)
                        // Actually for x², the epigraph is convex so we need:
                        // w ≥ x² (convex), but we can only approximate linearly
                        // Use simple secant: w ≤ (xl+xu)*x - xl*xu
                        let (xl, xu) = (xi_iv.lb, xi_iv.ub);
                        let xi = milp_vars[*var_idx];

                        // Tangent lower bounds (convexity of x²)
                        let lb1 = LinearExpr::from_var(xi) * Rational::from_f64(2.0 * xl)
                            - LinearExpr::constant(Rational::from_f64(xl * xl));
                        milp_sys.add_constraint(LinearConstraint::ge(
                            LinearExpr::from_var(w) - lb1,
                        ));

                        let lb2 = LinearExpr::from_var(xi) * Rational::from_f64(2.0 * xu)
                            - LinearExpr::constant(Rational::from_f64(xu * xu));
                        milp_sys.add_constraint(LinearConstraint::ge(
                            LinearExpr::from_var(w) - lb2,
                        ));

                        // Secant upper bound
                        let ub = LinearExpr::from_var(xi) * Rational::from_f64(xl + xu)
                            - LinearExpr::constant(Rational::from_f64(xl * xu));
                        milp_sys.add_constraint(LinearConstraint::le(
                            LinearExpr::from_var(w) - ub,
                        ));

                        aux_vars.push((w, sqr(&xi_iv)));
                        let term_expr = LinearExpr::from_var(w) * Rational::from_f64(*coeff);
                        linear_expr = linear_expr + term_expr;
                    }
                }
            }

            // Add the constraint: linear_expr ≤ rhs or = rhs
            let constraint = if c.is_eq {
                LinearConstraint::eq(linear_expr - LinearExpr::constant(Rational::from_f64(c.rhs)))
            } else {
                LinearConstraint::le(linear_expr - LinearExpr::constant(Rational::from_f64(c.rhs)))
            };
            milp_sys.add_constraint(constraint);
        }

        // No objective (feasibility problem) — set dummy
        milp_sys.set_objective(Objective::None);

        // 3. Solve the MILP relaxation
        let result = solve_milp(&milp_sys, &self.config);

        match result {
            LpResult::Optimal(model, obj) => LpResult::Optimal(model, obj),
            LpResult::Infeasible => LpResult::Infeasible,
            LpResult::Unbounded => LpResult::Unbounded,
            other => other,
        }
    }
}

/// Helper functions for interval evaluation.

fn eval_interval(terms: &[MinlpTerm], intervals: &[Interval]) -> Interval {
    let mut result = Interval::new(0.0, 0.0);
    for term in terms {
        let term_iv = eval_term_interval(term, intervals);
        result = interval::add(&result, &term_iv);
    }
    result
}

fn eval_interval_except(terms: &[MinlpTerm], intervals: &[Interval], except_var: usize) -> Interval {
    let mut result = Interval::new(0.0, 0.0);
    for term in terms {
        match term {
            MinlpTerm::Linear(coeff, var) if *var == except_var => continue,
            _ => {}
        }
        let term_iv = eval_term_interval(term, intervals);
        result = interval::add(&result, &term_iv);
    }
    result
}

fn eval_term_interval(term: &MinlpTerm, intervals: &[Interval]) -> Interval {
    match term {
        MinlpTerm::Linear(coeff, var) => {
            let iv = intervals[*var];
            Interval::new(iv.lb * coeff, iv.ub * coeff)
        }
        MinlpTerm::Bilinear(coeff, i, j) => {
            let a = intervals[*i];
            let b = intervals[*j];
            let prod = interval::mul(&a, &b);
            Interval::new(prod.lb * coeff, prod.ub * coeff)
        }
        MinlpTerm::Square(coeff, var) => {
            let iv = intervals[*var];
            let sq = interval::sqr(&iv);
            Interval::new(sq.lb * coeff, sq.ub * coeff)
        }
    }
}

use interval::{mul, sqr, sub};

// ── helper: convert f64 to Rational ───────────────────────────────────

impl Rational {
    fn from_f64(x: f64) -> Self {
        if x == 0.0 { return Rational::ZERO; }
        if x.fract() == 0.0 { return Rational::new(x as i64, 1); }
        let scale = 1_000_000_000i64;
        Rational::new((x * scale as f64).round() as i64, scale)
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minlp::MinlpVar;

    #[test]
    fn test_simple_bilinear() {
        // min -x*y  s.t. x in [0,1], y in [0,1]
        // Optimal should be at x=1, y=1, obj=-1
        let problem = MinlpProblem {
            vars: vec![
                MinlpVar { name: "x".into(), lb: 0.0, ub: 1.0, is_integer: false },
                MinlpVar { name: "y".into(), lb: 0.0, ub: 1.0, is_integer: false },
            ],
            constraints: vec![],
        };
        let mut solver = MinlpSolver::new(Config::default());
        // Just check it doesn't crash and returns something
        let result = solver.solve(&problem);
        // Without an objective it should be feasible
        match result {
            LpResult::Optimal(_, _) => {} // ok
            LpResult::Feasible(_) => {}   // ok
            _ => panic!("Expected feasible, got {:?}", result),
        }
    }
}
