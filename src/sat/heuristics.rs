use crate::sat::Lit;

/// VSIDS variable activity scores.
#[derive(Debug, Clone)]
pub struct VarActivity {
    /// Activity score for each variable.
    scores: Vec<f64>,
    /// Current bump value (incremented each time a variable is involved in a conflict).
    bump: f64,
    /// Activity decay factor per conflict.
    decay: f64,
}

impl VarActivity {
    /// Create a new activity tracker for `var_count` variables.
    pub fn new(var_count: usize) -> Self {
        VarActivity {
            scores: vec![0.0; var_count],
            bump: 1.0,
            decay: 0.95,
        }
    }

    /// Bump the activity of a literal's variable.
    #[inline]
    pub fn bump_lit(&mut self, lit: Lit) {
        self.scores[lit.var] += self.bump;
    }

    /// Bump the activity of a variable directly.
    #[inline]
    pub fn bump_var(&mut self, var: usize) {
        self.scores[var] += self.bump;
    }

    /// Get the activity score for a variable.
    #[inline]
    pub fn score(&self, var: usize) -> f64 {
        self.scores[var]
    }

    /// Called after each conflict to decay all activities.
    #[inline]
    pub fn decay(&mut self) {
        self.bump /= self.decay;
    }

    /// Rescale scores when they get too large.
    pub fn rescale(&mut self) {
        if self.bump > 1e100 {
            for s in &mut self.scores {
                *s *= 1e-100;
            }
            self.bump *= 1e-100;
        }
    }

    /// Number of variables tracked.
    #[inline]
    pub fn var_count(&self) -> usize {
        self.scores.len()
    }

    /// Find the unassigned variable with the highest activity.
    ///
    /// `is_assigned` should return `true` for assigned variables.
    pub fn select_var<F>(&self, is_assigned: F) -> Option<usize>
    where
        F: Fn(usize) -> bool,
    {
        self.scores
            .iter()
            .enumerate()
            .filter(|&(v, _)| !is_assigned(v))
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(v, _)| v)
    }
}

/// Geometric restart scheduler.
///
/// Restarts occur after `base * factor^n` conflicts.
#[derive(Debug, Clone)]
pub struct RestartScheduler {
    /// Initial restart interval.
    base: f64,
    /// Geometric factor.
    factor: f64,
    /// Current restart limit (threshold on total conflicts).
    limit: f64,
    /// Number of restarts so far.
    restart_count: u64,
}

impl RestartScheduler {
    /// Create a new geometric restart scheduler.
    pub fn new(base: f64, factor: f64) -> Self {
        RestartScheduler {
            base,
            factor,
            limit: base,
            restart_count: 0,
        }
    }

    /// Check whether a restart is due, given the current conflict count.
    pub fn should_restart(&self, total_conflicts: u64) -> bool {
        total_conflicts as f64 >= self.limit
    }

    /// Advance to the next restart interval.
    pub fn next_interval(&mut self) {
        self.restart_count += 1;
        self.limit = self.base * self.factor.powf(self.restart_count as f64);
    }
}

impl Default for RestartScheduler {
    fn default() -> Self {
        RestartScheduler::new(100.0, 1.5)
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_activity_bump_and_decay() {
        let mut va = VarActivity::new(3);
        va.bump_var(1);
        assert!(va.score(1) > 0.0);
        let before = va.score(1);
        va.decay();
        // decay doesn't change existing scores, only the bump increment
        assert_eq!(va.score(1), before);
        // but subsequent bumps will be larger
        va.bump_var(1);
        assert!(va.score(1) > before);
    }

    #[test]
    fn test_select_var() {
        let mut va = VarActivity::new(5);
        va.bump_var(2);
        va.bump_var(2);
        va.bump_var(4);
        let picked = va.select_var(|v| v == 4);
        assert_eq!(picked, Some(2));
    }

    #[test]
    fn test_select_var_all_assigned() {
        let va = VarActivity::new(3);
        assert_eq!(va.select_var(|_| true), None);
    }

    #[test]
    fn test_restart_scheduler() {
        let mut rs = RestartScheduler::new(100.0, 2.0);
        assert!(!rs.should_restart(50));
        assert!(rs.should_restart(100));
        assert!(rs.should_restart(150));
        rs.next_interval();
        assert_eq!(rs.restart_count, 1);
        assert!(rs.should_restart(200)); // 100*2 = 200
        assert!(!rs.should_restart(150));
    }
}
