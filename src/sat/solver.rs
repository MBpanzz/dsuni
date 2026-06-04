use crate::core::{Config, Model, SolverResult};
use crate::sat::{Clause, ClauseDB, ClauseId, Lit, Reason, Trail, VarActivity, WatchList};

/// The main CDCL SAT solver.
pub struct Solver {
    /// Configuration (reserved for verbosity / timeouts).
    #[allow(dead_code)]
    config: Config,
    /// Number of variables.
    var_count: usize,
    /// Clause database.
    clauses: ClauseDB,
    /// Assignment: `assignment[var]` = `Some(true)` / `Some(false)` / `None`.
    assignment: Vec<Option<bool>>,
    /// For each variable, the trail index at which it was assigned (for quick lookup).
    assignment_trail_pos: Vec<Option<usize>>,
    /// Assignment trail.
    trail: Trail,
    /// Watch lists (two-watched literals).
    watch: WatchList,
    /// VSIDS variable activity.
    activity: VarActivity,
    /// Index into the trail for the next literal to propagate.
    propagation_queue: usize,
    /// Internal statistics.
    stats: SolverStats,
}

/// Internal solver statistics.
#[derive(Debug, Clone, Default)]
pub struct SolverStats {
    pub decisions: u64,
    pub propagations: u64,
    pub conflicts: u64,
    pub restarts: u64,
}

impl Solver {
    /// Create a new SAT solver with the given configuration.
    pub fn new(config: Config) -> Self {
        Solver {
            config,
            var_count: 0,
            clauses: ClauseDB::new(),
            assignment: Vec::new(),
            assignment_trail_pos: Vec::new(),
            trail: Trail::new(),
            watch: WatchList::new(0),
            activity: VarActivity::new(0),
            propagation_queue: 0,
            stats: SolverStats::default(),
        }
    }

    // ── clause management ─────────────────────────────────────────────

    /// Add a clause (as a list of literals) to the solver.
    ///
    /// Automatically extends variable count and sets up watches.
    /// Unit clauses are immediately enqueued for propagation.
    /// Empty clauses immediately make the problem unsatisfiable.
    pub fn add_clause(&mut self, lits: &[Lit]) {
        if lits.is_empty() {
            // Empty clause → UNSAT. We'll detect this at solve time.
            self.clauses.alloc(Clause::new(vec![]));
            return;
        }

        // Ensure enough variable slots exist.
        let max_var = lits.iter().map(|l| l.var).max().unwrap_or(0);
        self.ensure_var_capacity(max_var + 1);

        // Unit clause: immediately enqueue.
        if lits.len() == 1 {
            let lit = lits[0];
            // Check if the literal's variable is already assigned.
            match self.assignment[lit.var] {
                Some(v) if v == lit.sign => {
                    // Already satisfied — clause is redundant, skip.
                    return;
                }
                Some(v) if v != lit.sign => {
                    // Conflict: variable assigned opposite.
                    // Record that we have a conflicting clause (empty = conflict sentinel).
                    self.clauses.alloc(Clause::new(vec![]));
                    return;
                }
                _ => {
                    // Unassigned: enqueue at level 0.
                    let clause = Clause::new(lits.to_vec());
                    let id = self.clauses.alloc(clause);
                    self.assign_lit(lit, Reason::Propagation(id));
                }
            }
            return;
        }

        // Clause with 2+ literals: set up two-watched literals.
        let clause = Clause::new(lits.to_vec());
        let id = self.clauses.alloc(clause);
        self.watch.watch(lits[0], id);
        self.watch.watch(lits[1], id);
    }

    /// Ensure capacity for at least `count` variables.
    fn ensure_var_capacity(&mut self, count: usize) {
        if count > self.var_count {
            self.var_count = count;
            self.assignment.resize(count, None);
            self.assignment_trail_pos.resize(count, None);
            self.watch = WatchList::new(count);
            self.activity = VarActivity::new(count);
        }
    }

    // ── assignment management ─────────────────────────────────────────

    /// Check whether a variable is currently assigned.
    #[inline]
    pub fn is_assigned(&self, var: usize) -> bool {
        self.assignment[var].is_some()
    }

    /// Check whether a literal is currently assigned **true**.
    #[inline]
    pub fn lit_true(&self, lit: Lit) -> bool {
        self.assignment[lit.var] == Some(lit.sign)
    }

    /// Check whether a literal is currently assigned **false**.
    #[inline]
    pub fn lit_false(&self, lit: Lit) -> bool {
        self.assignment[lit.var] == Some(!lit.sign)
    }

    /// The value of a literal: `Some(true)` (true), `Some(false)` (false), `None` (unassigned).
    #[inline]
    pub fn lit_value(&self, lit: Lit) -> Option<bool> {
        self.assignment[lit.var].map(|v| v == lit.sign)
    }

    /// Assign a literal and record the reason on the trail.
    fn assign_lit(&mut self, lit: Lit, reason: Reason) {
        debug_assert!(!self.is_assigned(lit.var));
        self.assignment[lit.var] = Some(lit.sign);
        self.assignment_trail_pos[lit.var] = Some(self.trail.len());

        match reason {
            Reason::Decision => {
                self.trail.push_decision(lit);
            }
            Reason::Propagation(_) => {
                self.trail.push_propagated(lit, reason);
            }
        }
    }

    /// Backtrack to the given decision level, unassigning variables.
    ///
    /// Removes all assignments made at levels > `level`.
    fn unassign_to_level(&mut self, level: super::trail::DecisionLevel) {
        if level + 1 >= self.trail.num_levels() {
            return; // nothing to do
        }

        // First index of entries at levels past `level`.
        let start_idx = self.trail.level_start(level + 1);

        // Collect variable ids to unassign before truncating.
        let vars_to_clear: Vec<usize> = (start_idx..self.trail.len())
            .map(|i| self.trail.get(i).lit.var)
            .collect();

        self.trail.backtrack(level);
        self.propagation_queue = self.trail.len().min(self.propagation_queue);

        for var in vars_to_clear {
            self.assignment[var] = None;
            self.assignment_trail_pos[var] = None;
        }
    }

    // ── propagation ───────────────────────────────────────────────────

    /// Unit propagation using two-watched literals.
    ///
    /// Processes each newly assigned literal from the propagation queue.
    /// For each clause watching the negated literal, tries to find a new
    /// non-false literal to watch. If none exists, the clause is either
    /// conflicting (all false) or unit (one unassigned, which is forced).
    ///
    /// Returns `Some(ClauseId)` of the conflicting clause, or `None`.
    fn propagate(&mut self) -> Option<ClauseId> {
        while self.propagation_queue < self.trail.len() {
            let p = self.trail.get(self.propagation_queue).lit;
            self.propagation_queue += 1;
            let not_p = p.not();

            // Collect clause IDs watching ¬p (clone to avoid borrow issues).
            let watch_ids: Vec<ClauseId> = self.watch.watches(not_p).to_vec();
            if watch_ids.is_empty() {
                continue;
            }

            for clause_id in watch_ids {
                let clause_false = self.process_watched_clause(clause_id, not_p)?;
                if clause_false {
                    // Clause is falsified — conflict
                    return Some(clause_id);
                }
            }
        }
        None
    }

    /// Process a clause that watches `false_lit` (which just became false).
    ///
    /// Returns `Ok(true)` if the clause is entirely false (conflict).
    /// Returns `Ok(false)` if the clause was handled (watch moved or satisfied).
    /// Returns `Err(())` on internal error (unused).
    fn process_watched_clause(&mut self, clause_id: ClauseId, false_lit: Lit) -> Option<bool> {
        let clause = self.clauses.get(clause_id);
        if clause.deleted {
            return Some(false);
        }

        // Try to find a non-false literal to watch instead of `false_lit`.
        // We look for any literal L where L != false_lit and L is not false.
        let mut found_replacement = false;
        let replacement_lit = {
            let mut repl = None;
            for &lit in &clause.lits {
                if lit == false_lit {
                    continue;
                }
                if !self.lit_false(lit) {
                    // This literal is either true (clause satisfied) or unassigned.
                    // It can serve as a replacement watch.
                    repl = Some(lit);
                    found_replacement = true;
                    break;
                }
            }
            repl
        };

        if found_replacement {
            // Move the watch: add watch on the replacement, remove from `false_lit`.
            let replacement = replacement_lit.unwrap();
            self.watch.watch(replacement, clause_id);
            // Remove old watch (lazy — just leave it, it'll be skipped next time
            // because the clause is no longer watching `false_lit`).
            // Actually, to avoid re-processing stale entries, we remove it.
            self.watch.remove_watch(false_lit, clause_id);
            Some(false) // no conflict
        } else {
            // All literals are false → conflict
            // (If exactly one is unassigned, the propagation happens when that
            // literal's negation becomes false. But we can also detect the unit
            // case here.)
            //
            // Actually, there might be a true/unassigned literal. If all are false
            // except false_lit which is now false too → conflict.
            // If there's exactly one unassigned literal, that literal must be true.
            let mut unassigned: Option<Lit> = None;
            for &lit in &clause.lits {
                if lit == false_lit {
                    continue;
                }
                if self.lit_value(lit) == Some(true) {
                    // Clause is already satisfied, no conflict.
                    self.watch.remove_watch(false_lit, clause_id);
                    return Some(false);
                }
                if self.lit_value(lit).is_none() {
                    if unassigned.is_some() {
                        // More than one unassigned — clause is neither unit nor conflict.
                        // This shouldn't happen in standard watched-literals because
                        // we only get here if all non-false_lit literals are false.
                        // But with lazy updates it can. Just keep watching false_lit.
                        return Some(false);
                    }
                    unassigned = Some(lit);
                }
            }

            if let Some(unit_lit) = unassigned {
                // Unit propagation: this literal must be true.
                self.assign_lit(unit_lit, Reason::Propagation(clause_id));
                self.stats.propagations += 1;
                self.watch.remove_watch(false_lit, clause_id);
                Some(false)
            } else {
                // All literals false → conflict.
                Some(true)
            }
        }
    }

    // ── conflict analysis ─────────────────────────────────────────────

    /// Analyze a conflict using 1UIP (First Unique Implication Point).
    ///
    /// 1. Resolve the conflict clause backwards along the trail
    /// 2. Stop when only one literal from the current decision level remains
    /// 3. The resulting clause is the learned asserting clause
    /// 4. Compute backtrack level (second-highest decision level)
    /// 5. Add the learned clause, backtrack, and assert the 1UIP literal
    fn analyze_and_learn(&mut self, conflict_clause: ClauseId) {
        let curr_level = self.trail.current_level();
        let mut seen = vec![false; self.var_count];
        let mut learn_lits: Vec<Lit> = Vec::new();
        let mut pathc = 0usize; // count of unresolved literals at current level
        let mut bt_level = 0usize; // backtrack level

        // Start from the last literal on the trail.
        let mut trail_idx = self.trail.len().wrapping_sub(1);

        // Add all literals from the conflicting clause.
        {
            let clause = self.clauses.get(conflict_clause);
            for &lit in &clause.lits {
                self.activity.bump_lit(lit);
                if !seen[lit.var] {
                    seen[lit.var] = true;
                    if self.trail
                        .get(self.assignment_trail_pos[lit.var].unwrap())
                        .level
                        == curr_level
                    {
                        pathc += 1;
                    } else if self.trail
                        .get(self.assignment_trail_pos[lit.var].unwrap())
                        .level
                        > bt_level
                    {
                        bt_level = self.trail
                            .get(self.assignment_trail_pos[lit.var].unwrap())
                            .level;
                    }
                    learn_lits.push(lit);
                }
            }
        }

        // Resolve until we reach the 1UIP.
        let asserting_lit = loop {
            // Find the next variable on the trail that is in `seen`.
            while trail_idx > 0 {
                let e = self.trail.get(trail_idx);
                if seen[e.lit.var] {
                    break;
                }
                trail_idx = trail_idx.wrapping_sub(1);
            }

            let entry = self.trail.get(trail_idx);
            seen[entry.lit.var] = false;
            pathc = pathc.saturating_sub(1);

            match entry.reason {
                Reason::Decision => {
                    // Reached a decision — this is the 1UIP.
                    break entry.lit;
                }
                Reason::Propagation(reason_clause) => {
                    // Resolve with the reason clause.
                    let reason = self.clauses.get(reason_clause);
                    for &rlit in &reason.lits {
                        self.activity.bump_lit(rlit);
                        if !seen[rlit.var] {
                            seen[rlit.var] = true;
                            let rlevel = self.trail
                                .get(self.assignment_trail_pos[rlit.var].unwrap())
                                .level;
                            if rlevel == curr_level {
                                pathc += 1;
                            } else if rlevel > bt_level {
                                bt_level = rlevel;
                            }
                            learn_lits.push(rlit);
                        }
                    }
                }
            }

            if trail_idx == 0 || pathc == 0 {
                break entry.lit;
            }
            trail_idx = trail_idx.wrapping_sub(1);
        };

        // The 1UIP literal is the trail entry literal.
        // For the learned clause, we need the NEGATION of the 1UIP as the
        // asserting literal (since the conflict is that the 1UIP's negation
        // was falsified by the 1UIP being true).
        let asserting = asserting_lit.not();

        // Build learned clause with the asserting literal first.
        let mut learned_lits = vec![asserting];
        for &l in &learn_lits {
            if l != asserting {
                learned_lits.push(l);
            }
        }

        // Add the learned clause.
        let learned_id = self.clauses.alloc(Clause::learned(learned_lits));

        // Set up two-watched literals for the learned clause.
        let learned = self.clauses.get(learned_id);
        if learned.len() >= 2 {
            self.watch.watch(learned.lits[0], learned_id);
            self.watch.watch(learned.lits[1], learned_id);
        }

        // Backtrack.
        self.unassign_to_level(bt_level);

        // Assert the negated 1UIP literal (it will be unit after backtrack).
        self.assign_lit(asserting, Reason::Propagation(learned_id));
        self.stats.propagations += 1;
    }

    // ── decision ──────────────────────────────────────────────────────

    /// Choose an unassigned variable and assign it (decision).
    fn decide(&mut self) {
        let var = self
            .activity
            .select_var(|v| self.is_assigned(v));
        if let Some(v) = var {
            // Decide the variable with the preferred phase (default: true).
            let lit = Lit::positive(v);
            self.assign_lit(lit, Reason::Decision);
            self.stats.decisions += 1;
        }
    }

    // ── solve ─────────────────────────────────────────────────────────

    /// Solve the current CNF formula.
    pub fn solve(&mut self) -> SolverResult {
        // Check for empty clause (added during unit-clause conflict).
        for id in 0..self.clauses.len() {
            let c = self.clauses.get(id);
            if !c.deleted && c.lits.is_empty() {
                return SolverResult::Unsat;
            }
        }

        // Main CDCL loop
        loop {
            // Unit propagation
            let conflict = self.propagate();
            if let Some(clause_id) = conflict {
                if self.trail.current_level() == 0 {
                    // Conflict at level 0 → UNSAT
                    return SolverResult::Unsat;
                }

                // Conflict analysis and backtrack
                self.stats.conflicts += 1;
                self.analyze_and_learn(clause_id);
                self.activity.decay();
                continue;
            }

            // All variables assigned → SAT
            if self.all_assigned() {
                let model = self.build_model();
                return SolverResult::Sat(model);
            }

            // Make a decision
            self.decide();
        }
    }

    /// Check whether all variables are assigned.
    fn all_assigned(&self) -> bool {
        (0..self.var_count).all(|v| self.assignment[v].is_some())
    }

    /// Build a model from the current assignment.
    fn build_model(&self) -> Model {
        use crate::core::{Assignment, Value};

        let mut a = Assignment::new();
        for (var, val) in self.assignment.iter().enumerate() {
            if let Some(v) = val {
                a.set(crate::core::Var::new(var), Value::Bool(*v));
            }
        }
        Model::new(a)
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_solver() {
        let solver = Solver::new(Config::default());
        assert_eq!(solver.var_count, 0);
    }

    #[test]
    fn test_add_clause_extends_vars() {
        let mut solver = Solver::new(Config::default());
        solver.add_clause(&[Lit::positive(0), Lit::negative(1)]);
        assert_eq!(solver.var_count, 2);
        assert!(solver.assignment.len() >= 2);
    }

    #[test]
    fn test_lit_assigned() {
        let mut solver = Solver::new(Config::default());
        // Add a binary clause (not unit) so no auto-assignment happens
        solver.add_clause(&[Lit::positive(0), Lit::positive(1)]);
        assert!(!solver.is_assigned(0));
        solver.assign_lit(Lit::positive(0), Reason::Propagation(0));
        assert!(solver.lit_true(Lit::positive(0)));
        assert!(solver.lit_false(Lit::negative(0)));
        assert!(!solver.is_assigned(1));
    }

    #[test]
    fn test_unsat_empty_clause() {
        let mut solver = Solver::new(Config::default());
        solver.add_clause(&[]); // empty clause = UNSAT
        let result = solver.solve();
        assert!(result.is_unsat());
    }
}
