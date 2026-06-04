use crate::sat::{ClauseId, Lit};

/// The reason why a variable was assigned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reason {
    /// Decision — the variable was chosen heuristically.
    Decision,
    /// Unit propagation from a clause.
    Propagation(ClauseId),
}

/// An entry on the assignment trail.
#[derive(Debug, Clone, Copy)]
pub struct TrailEntry {
    /// The literal that was assigned.
    pub lit: Lit,
    /// The decision level at which this assignment was made.
    pub level: DecisionLevel,
    /// The reason for the assignment.
    pub reason: Reason,
}

/// A decision level (0 = top-level, >0 = decision levels).
pub type DecisionLevel = usize;

/// The trail records the order of assignments during CDCL search.
///
/// Semantics:
/// - `boundaries[l]` = index of first entry at decision level `l`
/// - `entries[boundaries[l] .. boundaries[l+1]]` span level `l`
/// - `boundaries` always has at least one entry (level 0 starts at 0)
///
/// A decision is made at the **current** level, then a new level boundary
/// is pushed so that subsequent propagations belong to the new level.
#[derive(Debug, Clone)]
pub struct Trail {
    /// Ordered list of assigned literals with their reasons.
    entries: Vec<TrailEntry>,
    /// For each decision level, the index in `entries` where it begins.
    /// `boundaries[0]` = 0 (level 0 starts at index 0).
    boundaries: Vec<usize>,
}

impl Trail {
    /// Create a new empty trail.
    pub fn new() -> Self {
        Trail {
            entries: Vec::new(),
            boundaries: vec![0],
        }
    }

    /// The current (highest) decision level.
    #[inline]
    pub fn current_level(&self) -> DecisionLevel {
        self.boundaries.len() - 1
    }

    /// The number of decision levels (including level 0).
    #[inline]
    pub fn num_levels(&self) -> usize {
        self.boundaries.len()
    }

    /// Index of the first entry at the given level.
    #[inline]
    pub fn level_start(&self, level: DecisionLevel) -> usize {
        self.boundaries[level]
    }

    /// Make a decision: push a new decision level boundary, then record
    /// the decision literal at the NEW level.
    ///
    /// Returns the new current level.
    pub fn push_decision(&mut self, lit: Lit) -> DecisionLevel {
        let new_level = self.current_level() + 1;
        // Boundaries mark the START of each level.
        // So boundary for the new level = entries.len() (before pushing).
        self.boundaries.push(self.entries.len());
        self.entries.push(TrailEntry {
            lit,
            level: new_level,
            reason: Reason::Decision,
        });
        new_level
    }

    /// Push a propagated assignment (stays at current decision level).
    pub fn push_propagated(&mut self, lit: Lit, reason: Reason) {
        debug_assert!(matches!(reason, Reason::Propagation(_)));
        self.entries.push(TrailEntry {
            lit,
            level: self.current_level(),
            reason,
        });
    }

    /// Number of assignments at the current decision level.
    pub fn current_level_size(&self) -> usize {
        let cl = self.current_level();
        self.entries.len() - self.boundaries[cl]
    }

    /// Backtrack to the given decision level: remove all assignments
    /// made at levels > `target_level`.
    ///
    /// Returns the number of entries removed.
    pub fn backtrack(&mut self, target_level: DecisionLevel) -> usize {
        debug_assert!(
            target_level < self.boundaries.len(),
            "target level {target_level} >= num levels {}",
            self.boundaries.len()
        );
        // Keep entries up to (but not including) the start of target_level+1.
        let keep_len = if target_level + 1 < self.boundaries.len() {
            self.boundaries[target_level + 1]
        } else {
            self.entries.len()
        };
        let removed = self.entries.len() - keep_len;
        self.entries.truncate(keep_len);
        self.boundaries.truncate(target_level + 1);
        removed
    }

    /// The number of assigned literals on the trail.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the trail is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Access the trail entry at the given index.
    #[inline]
    pub fn get(&self, index: usize) -> &TrailEntry {
        &self.entries[index]
    }

    /// Access the last trail entry.
    #[inline]
    pub fn last(&self) -> Option<&TrailEntry> {
        self.entries.last()
    }

    /// Iterate over all trail entries.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &TrailEntry> {
        self.entries.iter()
    }

    /// Iterate over entries at the given decision level.
    pub fn iter_level(&self, level: DecisionLevel) -> &[TrailEntry] {
        let start = self.boundaries[level];
        let end = if level + 1 < self.boundaries.len() {
            self.boundaries[level + 1]
        } else {
            self.entries.len()
        };
        &self.entries[start..end]
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_trail() {
        let t = Trail::new();
        assert_eq!(t.current_level(), 0);
        assert!(t.is_empty());
    }

    #[test]
    fn test_push_decision() {
        let mut t = Trail::new();
        let new_level = t.push_decision(Lit::positive(0));
        assert_eq!(new_level, 1);
        assert_eq!(t.current_level(), 1);
        assert_eq!(t.len(), 1);
        // The decision entry is at level 1 (the new level)
        assert_eq!(t.get(0).level, 1);
        assert_eq!(t.get(0).reason, Reason::Decision);
    }

    #[test]
    fn test_decision_and_propagations() {
        let mut t = Trail::new();
        t.push_propagated(Lit::positive(0), Reason::Propagation(0));
        t.push_decision(Lit::positive(1));
        t.push_propagated(Lit::positive(2), Reason::Propagation(1));

        // push_decision pushes boundary first, so new level = 1.
        // Only one decision made, so current_level = 1.
        assert_eq!(t.current_level(), 1);
        assert_eq!(t.len(), 3);

        // With push_decision pushing the boundary FIRST:
        // propagations before the decision go to level 0 (if no prior decision)
        // the decision goes to level 1
        // subsequent propagations go to level 1 (now current)
        assert_eq!(t.get(0).lit, Lit::positive(0));
        assert_eq!(t.get(0).level, 0); // propagated at level 0
        assert_eq!(t.get(1).lit, Lit::positive(1));
        assert_eq!(t.get(1).level, 1); // decision at level 1
        assert_eq!(t.get(2).lit, Lit::positive(2));
        assert_eq!(t.get(2).level, 1); // propagated at level 1
    }

    #[test]
    fn test_backtrack() {
        let mut t = Trail::new();
        t.push_propagated(Lit::positive(0), Reason::Propagation(1));
        // boundaries: [0], entries: [lit0]
        t.push_decision(Lit::positive(1));
        // boundaries: [0, 1], entries: [lit0, lit1]
        t.push_propagated(Lit::positive(2), Reason::Propagation(2));
        // entries: [lit0, lit1, lit2]
        t.push_decision(Lit::positive(3));
        // boundaries: [0, 1, 3], entries: [lit0, lit1, lit2, lit3]

        assert_eq!(t.current_level(), 2);
        assert_eq!(t.len(), 4);

        // Backtrack to level 1: keep entries < boundaries[2] = < index 3
        let removed = t.backtrack(1);
        assert_eq!(removed, 1); // lit3 removed
        assert_eq!(t.current_level(), 1);
        assert_eq!(t.len(), 3); // lit0, lit1, lit2 remain
    }

    #[test]
    fn test_iter_level() {
        let mut t = Trail::new();
        t.push_propagated(Lit::positive(0), Reason::Propagation(0));
        t.push_decision(Lit::positive(1));
        t.push_propagated(Lit::positive(2), Reason::Propagation(1));

        // With new semantics:
        // boundaries after push_decision: [0, 1]
        // Level 0: entries [0, 1) = [lit0] (propagated)
        // Level 1: entries [1, 3) = [lit1(decision), lit2(prop)]
        assert_eq!(t.iter_level(0).len(), 1);
        assert_eq!(t.iter_level(1).len(), 2);
    }

    #[test]
    fn test_backtrack_to_zero() {
        let mut t = Trail::new();
        t.push_propagated(Lit::positive(0), Reason::Propagation(1));
        t.push_decision(Lit::positive(1));
        t.push_propagated(Lit::positive(2), Reason::Propagation(2));

        // After push_decision(lit1): boundaries = [0, 1], entries = [lit0, lit1]
        // After push_propagated(lit2): entries = [lit0, lit1, lit2]
        // Level 0: [0, 1) = [lit0]
        // Level 1: [1, 3) = [lit1, lit2]
        let removed = t.backtrack(0);
        // Keep entries < boundaries[1] = < index 1 → keep only lit0
        assert_eq!(removed, 2); // lit1, lit2 removed
        assert_eq!(t.current_level(), 0);
        assert_eq!(t.len(), 1); // lit0 remains (it was level 0)
    }

    #[test]
    fn test_push_decision_returns_new_level() {
        let mut t = Trail::new();
        let lvl1 = t.push_decision(Lit::positive(0));
        assert_eq!(lvl1, 1);
        let lvl2 = t.push_decision(Lit::positive(1));
        assert_eq!(lvl2, 2);
        assert_eq!(t.current_level(), 2);
    }
}
