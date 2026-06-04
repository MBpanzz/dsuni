use crate::cp::dom::IntDomain;

/// Result of propagation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropResult {
    /// Propagation completed (may or may not have pruned).
    Success,
    /// Domain became empty (constraint cannot be satisfied).
    Failure,
}

/// A propagator: prunes domains based on a constraint.
#[derive(Debug, Clone)]
pub enum Propagator {
    Eq(usize, usize),
    Neq(usize, usize),
    Lt(usize, usize),
    Le(usize, usize, i64),
    Linear(Vec<(i64, usize)>, i64, crate::cp::CmpRel),
    AllDifferent(Vec<usize>),
}

impl Propagator {
    pub fn variables(&self) -> Vec<usize> {
        match self {
            Propagator::Eq(a, b) | Propagator::Neq(a, b) | Propagator::Lt(a, b) => vec![*a, *b],
            Propagator::Le(a, b, _) => vec![*a, *b],
            Propagator::Linear(terms, _, _) => terms.iter().map(|(_, v)| *v).collect(),
            Propagator::AllDifferent(vars) => vars.clone(),
        }
    }

    /// Run propagation. Returns Success (even if nothing changed).
    /// The caller (propagate_all) detects fixed point by rerunning until
    /// no propagator reports a change.
    pub fn propagate(&self, domains: &mut [IntDomain]) -> PropResult {
        match self {
            Propagator::Eq(x, y) => propagate_eq(domains, *x, *y),
            Propagator::Neq(x, y) => propagate_neq(domains, *x, *y),
            Propagator::Lt(x, y) => propagate_lt(domains, *x, *y),
            Propagator::Le(x, y, c) => propagate_le(domains, *x, *y, *c),
            Propagator::Linear(terms, rhs, rel) => propagate_linear(domains, terms, *rhs, *rel),
            Propagator::AllDifferent(vars) => propagate_all_diff(domains, vars),
        }
    }
}

/// Take a snapshot of relevant domains.
fn snapshot(domains: &[IntDomain], vars: &[usize]) -> Vec<(i64, i64)> {
    vars.iter().map(|&v| (domains[v].min(), domains[v].max())).collect()
}

/// Check if domains changed from a snapshot.
fn changed(domains: &[IntDomain], vars: &[usize], snap: &[(i64, i64)]) -> bool {
    vars.iter().zip(snap.iter()).any(|(&v, &(mn, mx))| {
        domains[v].min() != mn || domains[v].max() != mx
    })
}

fn propagate_eq(domains: &mut [IntDomain], x: usize, y: usize) -> PropResult {
    let snap = snapshot(domains, &[x, y]);
    let lb = domains[x].min().max(domains[y].min());
    let ub = domains[x].max().min(domains[y].max());
    if lb > ub { return PropResult::Failure; }
    if !domains[x].prune_lb(lb) || !domains[y].prune_lb(lb) ||
       !domains[x].prune_ub(ub) || !domains[y].prune_ub(ub) {
        return PropResult::Failure;
    }
    if changed(domains, &[x, y], &snap) { PropResult::Success } else { PropResult::Success }
}

fn propagate_neq(domains: &mut [IntDomain], x: usize, y: usize) -> PropResult {
    if let (Some(vx), Some(vy)) = (domains[x].value(), domains[y].value()) {
        if vx == vy { return PropResult::Failure; }
    }
    if let Some(vx) = domains[x].value() {
        if domains[y].contains(vx) && !domains[y].remove(vx) {
            return PropResult::Failure;
        }
    }
    if let Some(vy) = domains[y].value() {
        if domains[x].contains(vy) && !domains[x].remove(vy) {
            return PropResult::Failure;
        }
    }
    PropResult::Success
}

fn propagate_lt(domains: &mut [IntDomain], x: usize, y: usize) -> PropResult {
    let max_x = domains[x].max();
    let min_y = domains[y].min();
    if !domains[x].prune_ub(min_y - 1) || !domains[y].prune_lb(max_x + 1) {
        return PropResult::Failure;
    }
    PropResult::Success
}

fn propagate_le(domains: &mut [IntDomain], x: usize, y: usize, c: i64) -> PropResult {
    if !domains[x].prune_ub(domains[y].max() + c) ||
       !domains[y].prune_lb(domains[x].min() - c) {
        return PropResult::Failure;
    }
    PropResult::Success
}

fn propagate_linear(
    domains: &mut [IntDomain],
    terms: &[(i64, usize)],
    rhs: i64,
    rel: crate::cp::CmpRel,
) -> PropResult {
    for &(coeff, var) in terms {
        let (min_sum, max_sum) = min_max_sum(domains, terms, var);
        match rel {
            crate::cp::CmpRel::Eq => {
                let min_x = rhs - max_sum;
                let max_x = rhs - min_sum;
                if coeff > 0 {
                    if !domains[var].prune_lb(div_ceil(min_x, coeff)) { return PropResult::Failure; }
                    if !domains[var].prune_ub(div_floor(max_x, coeff)) { return PropResult::Failure; }
                } else if coeff < 0 {
                    if !domains[var].prune_lb(div_ceil(max_x, coeff)) { return PropResult::Failure; }
                    if !domains[var].prune_ub(div_floor(min_x, coeff)) { return PropResult::Failure; }
                }
            }
            crate::cp::CmpRel::Le => {
                if coeff > 0 {
                    if !domains[var].prune_ub(div_floor(rhs - min_sum, coeff)) { return PropResult::Failure; }
                } else if coeff < 0 {
                    if !domains[var].prune_lb(div_ceil(rhs - min_sum, coeff)) { return PropResult::Failure; }
                }
            }
            crate::cp::CmpRel::Ge => {
                if coeff > 0 {
                    if !domains[var].prune_lb(div_ceil(rhs - max_sum, coeff)) { return PropResult::Failure; }
                } else if coeff < 0 {
                    if !domains[var].prune_ub(div_floor(rhs - max_sum, coeff)) { return PropResult::Failure; }
                }
            }
        }
    }
    PropResult::Success
}

fn div_ceil(a: i64, b: i64) -> i64 {
    if b < 0 { div_ceil(-a, -b) }
    else if a >= 0 { (a + b - 1) / b }
    else { a / b }
}

fn div_floor(a: i64, b: i64) -> i64 {
    if b < 0 { div_floor(-a, -b) }
    else if a >= 0 { a / b }
    else { (a - b + 1) / b }
}

fn min_max_sum(domains: &[IntDomain], terms: &[(i64, usize)], exclude: usize) -> (i64, i64) {
    let mut min_sum = 0i64;
    let mut max_sum = 0i64;
    for &(coeff, var) in terms {
        if var == exclude { continue; }
        if coeff > 0 {
            min_sum += coeff * domains[var].min();
            max_sum += coeff * domains[var].max();
        } else {
            min_sum += coeff * domains[var].max();
            max_sum += coeff * domains[var].min();
        }
    }
    (min_sum, max_sum)
}

fn propagate_all_diff(domains: &mut [IntDomain], vars: &[usize]) -> PropResult {
    for i in 0..vars.len() {
        if let Some(vi) = domains[vars[i]].value() {
            for j in 0..vars.len() {
                if i != j && domains[vars[j]].contains(vi) {
                    if !domains[vars[j]].remove(vi) { return PropResult::Failure; }
                }
            }
        }
    }
    PropResult::Success
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_fixed() {
        let mut dom = vec![IntDomain::new(0, 10), IntDomain::new(0, 10)];
        let p = Propagator::Eq(0, 1);
        dom[0] = IntDomain::single(5);
        let r = p.propagate(&mut dom);
        assert_eq!(r, PropResult::Success);
        assert_eq!(dom[1].value(), Some(5));
    }

    #[test]
    fn test_neq_conflict() {
        let mut dom = vec![IntDomain::single(3), IntDomain::single(3)];
        let p = Propagator::Neq(0, 1);
        let r = p.propagate(&mut dom);
        assert_eq!(r, PropResult::Failure);
    }

    #[test]
    fn test_all_diff_endpoint() {
        // Test that all_diff removes fixed values from other domains
        // when the value is at an endpoint
        let mut dom = vec![
            IntDomain::single(0), // fixed at 0 (endpoint)
            IntDomain::new(0, 10),
            IntDomain::new(0, 10),
        ];
        let p = Propagator::AllDifferent(vec![0, 1, 2]);
        let r = p.propagate(&mut dom);
        assert_eq!(r, PropResult::Success);
        // remove(0) on [0,10]: val==min → min becomes 1 → [1,10]
        assert!(!dom[1].contains(0));
        assert!(!dom[2].contains(0));
    }
}
