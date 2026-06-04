use std::collections::BinaryHeap;
use crate::core::{Config, LpResult, Model, UnknownReason};
use crate::linear::{Bound, LinearSystem, Objective, Rational, Var};
use crate::lp::solve_lp;

/// A branch-and-bound node.
#[derive(Debug, Clone)]
struct Node {
    /// Additional bound constraints for this node.
    bounds: Vec<(Var, Bound)>,
    /// LP objective value at this node (for priority).
    bound: Option<f64>,
    /// Node depth.
    depth: usize,
}

/// Wrapper for ordering nodes in the priority queue (best-bound first).
#[derive(Debug, Clone)]
struct NodePriority {
    bound: f64,
    index: usize,
}

impl PartialEq for NodePriority {
    fn eq(&self, other: &Self) -> bool {
        self.bound == other.bound
    }
}

impl Eq for NodePriority {}

impl PartialOrd for NodePriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // For minimization: smaller bound = higher priority
        // For maximization: larger bound = higher priority
        other.bound.partial_cmp(&self.bound)
    }
}

impl Ord for NodePriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Branch and bound MILP solver.
#[derive(Debug, Clone)]
pub struct BnBSolver {
    config: Config,
}

impl BnBSolver {
    pub fn new(config: Config) -> Self {
        BnBSolver { config }
    }

    /// Solve a MILP using branch and bound.
    pub fn solve(&mut self, system: &LinearSystem) -> LpResult {
        // Identify integer variables
        let int_vars: Vec<Var> = system
            .vars()
            .iter()
            .filter(|v| system.is_integer(**v))
            .copied()
            .collect();

        if int_vars.is_empty() {
            // Pure LP — no integer variables
            return solve_lp(system, &self.config);
        }

        // Determine optimization direction
        let is_minimization = matches!(system.objective(), Objective::Minimise(_));

        // Solve the initial LP relaxation
        let initial_result = solve_lp(system, &self.config);

        let (mut incumbent_value, mut incumbent_model) = match &initial_result {
            LpResult::Optimal(model, obj) => {
                if all_integer_feasible(model, &int_vars) {
                    return initial_result;
                }
                // Use LP objective as initial bound for pruning.
                (*obj, None)
            }
            LpResult::Infeasible => return LpResult::Infeasible,
            _ => return initial_result,
        };

        // Priority queue for nodes
        let mut node_queue: BinaryHeap<NodePriority> = BinaryHeap::new();
        let mut nodes: Vec<Node> = Vec::new();

        // Add initial LP solution as a branching candidate
        if let LpResult::Optimal(model, obj) = &initial_result {
            // Find a fractional integer variable to branch on
            if let Some((var, val)) = find_fractional(model, &int_vars) {
                // Create two child nodes
                let floor_val = val.floor();
                let ceil_val = val.ceil();

                // Node 0: x ≤ floor(val)
                let mut bounds0 = Vec::new();
                bounds0.push((var, Bound::upper(rational(floor_val))));
                nodes.push(Node {
                    bounds: bounds0,
                    bound: None,
                    depth: 1,
                });
                node_queue.push(NodePriority { bound: *obj, index: 0 });

                // Node 1: x ≥ ceil(val)
                let mut bounds1 = Vec::new();
                bounds1.push((var, Bound::lower(rational(ceil_val))));
                nodes.push(Node {
                    bounds: bounds1,
                    bound: None,
                    depth: 1,
                });
                node_queue.push(NodePriority { bound: *obj, index: 1 });
            }
        }

        // BnB main loop
        let max_nodes = 1000; // Safety limit
        let mut nodes_processed = 0;

        while let Some(np) = node_queue.pop() {
            if nodes_processed >= max_nodes {
                break;
            }
            nodes_processed += 1;

            let node = nodes[np.index].clone();

            // Prune by bound
            if is_minimization && node.bound.unwrap_or(f64::NEG_INFINITY) >= incumbent_value {
                continue;
            }
            if !is_minimization && node.bound.unwrap_or(f64::INFINITY) <= incumbent_value {
                continue;
            }

            // Build the system with node's bounds as constraints
            let mut node_sys = system.clone();
            for &(var, ref bound) in &node.bounds {
                if let Some(lb) = bound.lower {
                    let expr = crate::linear::LinearExpr::from_var(var)
                        - crate::linear::LinearExpr::constant(lb);
                    node_sys.add_constraint(crate::linear::LinearConstraint::ge(expr));
                }
                if let Some(ub) = bound.upper {
                    let expr = crate::linear::LinearExpr::from_var(var)
                        - crate::linear::LinearExpr::constant(ub);
                    node_sys.add_constraint(crate::linear::LinearConstraint::le(expr));
                }
            }

            // Solve LP relaxation at this node
            let result = solve_lp(&node_sys, &self.config);

            match result {
                LpResult::Optimal(model, obj) => {
                    if all_integer_feasible(&model, &int_vars) {
                        // Feasible integer solution
                        if (is_minimization && obj < incumbent_value)
                            || (!is_minimization && obj > incumbent_value)
                        {
                            incumbent_value = obj;
                            incumbent_model = Some(model.clone());
                        }
                    } else {
                        // Branch
                        if let Some((var, val)) = find_fractional(&model, &int_vars) {
                            let floor_val = val.floor();
                            let ceil_val = val.ceil();
                            let base_idx = nodes.len();

                            let mut bounds_l = node.bounds.clone();
                            bounds_l.push((var, Bound::upper(rational(floor_val))));
                            nodes.push(Node {
                                bounds: bounds_l,
                                bound: Some(obj),
                                depth: node.depth + 1,
                            });
                            node_queue.push(NodePriority {
                                bound: obj,
                                index: base_idx,
                            });

                            let mut bounds_r = node.bounds.clone();
                            bounds_r.push((var, Bound::lower(rational(ceil_val))));
                            nodes.push(Node {
                                bounds: bounds_r,
                                bound: Some(obj),
                                depth: node.depth + 1,
                            });
                            node_queue.push(NodePriority {
                                bound: obj,
                                index: base_idx + 1,
                            });
                        }
                    }
                }
                LpResult::Infeasible => { /* prune */ }
                LpResult::Unbounded => {
                    if node.depth == 0 { return LpResult::Unbounded; }
                }
                _ => {}
            }
        }

        match incumbent_model {
            Some(model) => LpResult::Optimal(model, incumbent_value),
            None => {
                if nodes_processed >= max_nodes {
                    LpResult::Unknown(UnknownReason::ResourceLimit)
                } else {
                    LpResult::Infeasible
                }
            }
        }
    }
}

/// Find the first fractional integer variable in the model.
fn find_fractional(model: &Model, int_vars: &[Var]) -> Option<(Var, f64)> {
    for &v in int_vars {
        if let Some(val) = model.assignments.get(&v) {
            let f = val_to_f64(val);
            let frac = f - f.floor();
            if frac > 1e-10 && frac < 1.0 - 1e-10 {
                return Some((v, f));
            }
        }
    }
    None
}

/// Check if all integer variables have integer values.
fn all_integer_feasible(model: &Model, int_vars: &[Var]) -> bool {
    int_vars.iter().all(|v| {
        if let Some(val) = model.assignments.get(v) {
            let f = val_to_f64(val);
            (f - f.round()).abs() < 1e-8
        } else {
            false
        }
    })
}

/// Convert a Value to f64.
fn val_to_f64(val: &crate::core::Value) -> f64 {
    match val {
        crate::core::Value::Int(i) => *i as f64,
        crate::core::Value::Real(r) => *r,
        crate::core::Value::Bool(b) => {
            if *b { 1.0 } else { 0.0 }
        }
    }
}

/// Convert f64 to Rational (for branching bounds).
fn rational(x: f64) -> Rational {
    if x == 0.0 { return Rational::ZERO; }
    if x.fract() == 0.0 { return Rational::new(x as i64, 1); }
    let scale = 1_000_000_000i64;
    Rational::new((x * scale as f64).round() as i64, scale)
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear::{LinearConstraint, LinearExpr, LinearSystem, Objective};
    use crate::milp::solve_milp;

    #[test]
    fn test_pure_lp_via_milp() {
        // Pure LP (no integer vars) should fall through to LP solver
        let mut sys = LinearSystem::new();
        let x = sys.new_var(); // real by default
        sys.add_constraint(LinearConstraint::le(
            LinearExpr::from_var(x) - LinearExpr::from(5i64),
        ));
        sys.set_objective(Objective::Minimise(
            LinearExpr::from_var(x) * Rational::NEG_ONE,
        ));
        let result = solve_milp(&sys, &Config::default());
        assert!(matches!(result, LpResult::Optimal(_, _)));
    }

    #[test]
    fn test_simple_milp() {
        // min -x0, s.t. x0 <= 5, x0 integer
        let mut sys = LinearSystem::new();
        let x = sys.new_int_var();
        sys.add_constraint(LinearConstraint::le(
            LinearExpr::from_var(x) - LinearExpr::from(5i64),
        ));
        sys.add_constraint(LinearConstraint::ge(
            LinearExpr::from_var(x),
        ));
        sys.set_objective(Objective::Minimise(
            LinearExpr::from_var(x) * Rational::NEG_ONE,
        ));
        let result = solve_milp(&sys, &Config::default());
        match result {
            LpResult::Optimal(model, obj) => {
                assert!((obj - (-5.0)).abs() < 1e-5, "obj={}", obj);
                // x0 should be integer and ≤ 5
                if let Some(val) = model.assignments.get(&x) {
                    let f = val_to_f64(val);
                    assert!((f - 5.0).abs() < 1e-5 || (f - 0.0).abs() < 1e-5, "x0={}", f);
                }
            }
            other => panic!("Expected Optimal, got {:?}", other),
        }
    }
}
