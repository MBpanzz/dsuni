use crate::sat::{ClauseId, Lit};

/// Two-watched literal list.
///
/// For each literal index (`2*var + sign`), stores the set of clauses that
/// currently watch that literal.
#[derive(Debug, Clone)]
pub struct WatchList {
    /// `watchers[lit.index()]` = list of clause IDs watching that literal.
    watchers: Vec<Vec<ClauseId>>,
}

impl WatchList {
    /// Create a new watch list for `var_count` variables.
    pub fn new(var_count: usize) -> Self {
        WatchList {
            watchers: vec![Vec::new(); Lit::index_count(var_count)],
        }
    }

    /// Return the number of literal slots.
    #[inline]
    pub fn len(&self) -> usize {
        self.watchers.len()
    }

    /// Add a clause to the watch list of the given literal.
    #[inline]
    pub fn watch(&mut self, lit: Lit, clause: ClauseId) {
        self.watchers[lit.index()].push(clause);
    }

    /// Get all clauses watching the given literal.
    #[inline]
    pub fn watches(&self, lit: Lit) -> &[ClauseId] {
        &self.watchers[lit.index()]
    }

    /// Mutable access to the watcher list for a literal.
    #[inline]
    pub fn watches_mut(&mut self, lit: Lit) -> &mut Vec<ClauseId> {
        &mut self.watchers[lit.index()]
    }

    /// Remove a specific clause ID from the watch list of a literal.
    ///
    /// Returns `true` if the clause was found and removed.
    pub fn remove_watch(&mut self, lit: Lit, clause: ClauseId) -> bool {
        let w = &mut self.watchers[lit.index()];
        if let Some(pos) = w.iter().position(|&c| c == clause) {
            w.swap_remove(pos);
            true
        } else {
            false
        }
    }

    /// Remove a specific clause ID from ALL watch lists (when a clause is deleted).
    pub fn remove_clause(&mut self, clause: ClauseId) {
        for w in &mut self.watchers {
            w.retain(|&c| c != clause);
        }
    }
}
