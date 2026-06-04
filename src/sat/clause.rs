use crate::sat::Lit;

/// Index of a clause in the solver's clause database.
pub type ClauseId = usize;

/// A clause in CNF form — a disjunction of literals.
#[derive(Debug, Clone)]
pub struct Clause {
    /// The literals of the clause.
    /// For a learned clause, the first literal is the asserting literal.
    pub lits: Vec<Lit>,
    /// Whether this clause was learned during CDCL.
    pub is_learned: bool,
    /// Activity score for clause deletion heuristics.
    pub activity: f64,
    /// Whether the clause is deleted (marked as garbage).
    pub deleted: bool,
}

impl Clause {
    /// Create a new (non-learned) clause.
    pub fn new(lits: Vec<Lit>) -> Self {
        Clause {
            lits,
            is_learned: false,
            activity: 0.0,
            deleted: false,
        }
    }

    /// Create a new learned clause.
    pub fn learned(lits: Vec<Lit>) -> Self {
        Clause {
            lits,
            is_learned: true,
            activity: 0.0,
            deleted: false,
        }
    }

    /// The size (number of literals) of the clause.
    #[inline]
    pub fn len(&self) -> usize {
        self.lits.len()
    }

    /// Whether the clause is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.lits.is_empty()
    }

    /// Whether the clause is a unit clause (exactly one literal).
    #[inline]
    pub fn is_unit(&self) -> bool {
        self.lits.len() == 1
    }

    /// Mark this clause as deleted (garbage).
    #[inline]
    pub fn delete(&mut self) {
        self.deleted = true;
    }
}

/// The clause database: an arena of clauses indexed by [`ClauseId`].
#[derive(Debug, Clone)]
pub struct ClauseDB {
    clauses: Vec<Clause>,
}

impl ClauseDB {
    /// Create an empty clause database.
    pub fn new() -> Self {
        ClauseDB { clauses: Vec::new() }
    }

    /// Allocate (store) a clause and return its ID.
    pub fn alloc(&mut self, clause: Clause) -> ClauseId {
        let id = self.clauses.len();
        self.clauses.push(clause);
        id
    }

    /// Access a clause by ID.
    #[inline]
    pub fn get(&self, id: ClauseId) -> &Clause {
        &self.clauses[id]
    }

    /// Mutable access to a clause by ID.
    #[inline]
    pub fn get_mut(&mut self, id: ClauseId) -> &mut Clause {
        &mut self.clauses[id]
    }

    /// Number of clauses in the database (including deleted).
    #[inline]
    pub fn len(&self) -> usize {
        self.clauses.len()
    }

    /// Whether the database is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    /// Iterate over all non-deleted clauses.
    pub fn iter_active(&self) -> impl Iterator<Item = (ClauseId, &Clause)> {
        self.clauses
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.deleted)
    }

    /// Number of non-deleted clauses.
    pub fn active_count(&self) -> usize {
        self.clauses.iter().filter(|c| !c.deleted).count()
    }

    /// Garbage-collect deleted clauses by compacting the arena.
    /// Returns a mapping from old ID → new ID for surviving clauses.
    pub fn compact(&mut self) -> Vec<Option<ClauseId>> {
        let mut mapping = vec![None; self.clauses.len()];
        let mut write_idx = 0;
        for read_idx in 0..self.clauses.len() {
            if !self.clauses[read_idx].deleted {
                if write_idx != read_idx {
                    self.clauses.swap(write_idx, read_idx);
                }
                mapping[read_idx] = Some(write_idx);
                write_idx += 1;
            }
        }
        self.clauses.truncate(write_idx);
        mapping
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clause_new() {
        let c = Clause::new(vec![Lit::positive(0), Lit::negative(1)]);
        assert_eq!(c.len(), 2);
        assert!(!c.is_learned);
    }

    #[test]
    fn test_clause_learned() {
        let c = Clause::learned(vec![Lit::positive(0)]);
        assert!(c.is_learned);
        assert!(c.is_unit());
    }

    #[test]
    fn test_clause_db_alloc_and_get() {
        let mut db = ClauseDB::new();
        let id = db.alloc(Clause::new(vec![Lit::positive(0)]));
        assert_eq!(db.get(id).len(), 1);
    }

    #[test]
    fn test_clause_db_active_count() {
        let mut db = ClauseDB::new();
        db.alloc(Clause::new(vec![]));
        db.alloc(Clause::new(vec![]));
        assert_eq!(db.active_count(), 2);
        db.get_mut(1).delete();
        assert_eq!(db.active_count(), 1);
    }

    #[test]
    fn test_clause_delete_and_compact() {
        let mut db = ClauseDB::new();
        let id0 = db.alloc(Clause::new(vec![Lit::positive(0)]));
        let id1 = db.alloc(Clause::new(vec![Lit::positive(1)]));
        let id2 = db.alloc(Clause::new(vec![Lit::positive(2)]));
        db.get_mut(id1).delete();

        let mapping = db.compact();
        assert_eq!(db.active_count(), 2);
        assert_eq!(mapping[id0], Some(0));
        assert_eq!(mapping[id1], None); // deleted
        assert_eq!(mapping[id2], Some(1));
    }
}
