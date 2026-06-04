//! QF_UF theory solver using congruence closure.

use crate::smt::{TheoryAtom, TheoryCheckResult, TheorySolver};
use crate::smt::TheoryLit;

/// An equality atom: `a = b` where a, b are terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqAtom {
    pub id: usize,
    pub lhs: TermId,
    pub rhs: TermId,
}

impl TheoryAtom for EqAtom {
    fn id(&self) -> usize {
        self.id
    }
    fn label(&self) -> String {
        format!("t{} = t{}", self.lhs, self.rhs)
    }
    fn clone_box(&self) -> Box<dyn TheoryAtom> {
        Box::new(self.clone())
    }
}

/// Term identifier (index in the term DB).
pub type TermId = usize;

/// A function application term: `f(args...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncTerm {
    pub name: String,
    pub args: Vec<TermId>,
}

/// Format a function term as a string (for matching with SMT-LIB labels).
fn format_term(ft: &FuncTerm) -> String {
    if ft.args.is_empty() {
        ft.name.clone()
    } else {
        let args: Vec<String> = ft.args.iter().map(|&a| format!("t{}", a)).collect();
        format!("({} {})", ft.name, args.join(" "))
    }
}

/// The UF theory solver using congruence closure.
///
/// When atoms are set via `set_atoms`, the solver expects atom labels
/// of the form `t1 = t2` (from parsed SMT-LIB) and builds internal
/// term representations for congruence closure.
#[derive(Debug, Clone)]
pub struct UF {
    /// Parent pointer for union-find (term_id → parent).
    parent: Vec<TermId>,
    /// Rank for union-find.
    rank: Vec<usize>,
    /// For each term, its canonical representative.
    find_cache: Vec<TermId>,
    /// Terms stored by ID.
    func_terms: Vec<FuncTerm>,
    /// For each term, the set of parent terms (terms that use this term as argument).
    parents: Vec<Vec<TermId>>,
    /// Asserted equalities: (lhs, rhs) pairs that are asserted true.
    assertions: Vec<(TermId, TermId)>,
    /// Asserted disequalities.
    disequalities: Vec<(TermId, TermId)>,
    /// Next term ID.
    next_term: TermId,
    /// Stack for push/pop.
    stack: Vec<usize>,
    /// For each atom ID, the (lhs, rhs) term pair it represents.
    atom_terms: Vec<(TermId, TermId)>,
}

impl UF {
    pub fn new() -> Self {
        UF {
            parent: Vec::new(),
            rank: Vec::new(),
            find_cache: Vec::new(),
            func_terms: Vec::new(),
            parents: Vec::new(),
            assertions: Vec::new(),
            disequalities: Vec::new(),
            next_term: 0,
            stack: Vec::new(),
            atom_terms: Vec::new(),
        }
    }

    /// Create a new constant term (arity 0).
    pub fn new_const(&mut self) -> TermId {
        let id = self.next_term;
        self.next_term += 1;
        self.parent.push(id);
        self.rank.push(0);
        self.find_cache.push(id);
        self.func_terms.push(FuncTerm {
            name: format!("c{}", id),
            args: vec![],
        });
        self.parents.push(Vec::new());
        id
    }

    /// Create a new function application.
    pub fn new_app(&mut self, name: &str, args: Vec<TermId>) -> TermId {
        let id = self.next_term;
        self.next_term += 1;
        self.parent.push(id);
        self.rank.push(0);
        self.find_cache.push(id);
        self.func_terms.push(FuncTerm {
            name: name.to_string(),
            args: args.clone(),
        });
        self.parents.push(Vec::new());
        // Register as parent of each argument
        for &arg in &args {
            self.parents[arg].push(id);
        }
        id
    }

    /// Find the canonical representative of a term (with path compression).
    pub fn find(&mut self, x: TermId) -> TermId {
        let px = self.parent[x];
        if px != x {
            let root = self.find(px);
            self.parent[x] = root;
        }
        self.parent[x]
    }

    /// Union two terms.
    pub fn union(&mut self, x: TermId, y: TermId) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        if self.rank[rx] < self.rank[ry] {
            self.parent[rx] = ry;
        } else if self.rank[rx] > self.rank[ry] {
            self.parent[ry] = rx;
        } else {
            self.parent[ry] = rx;
            self.rank[rx] += 1;
        }
    }

    /// Check congruence: if two function applications have congruent arguments,
    /// merge their results.
    pub fn propagate_congruence(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            let n = self.func_terms.len();
            for i in 0..n {
                let fi = &self.func_terms[i].clone();
                if fi.args.is_empty() {
                    continue;
                }
                let ri = self.find(i);
                for j in (i + 1)..n {
                    let fj = &self.func_terms[j].clone();
                    if fi.name != fj.name || fi.args.len() != fj.args.len() {
                        continue;
                    }
                    // Check if args are congruent (same equivalence classes)
                    let congruent = fi.args.iter().zip(&fj.args).all(|(a, b)| {
                        self.find(*a) == self.find(*b)
                    });
                    if congruent {
                        let rj = self.find(j);
                        if ri != rj {
                            self.union(i, j);
                            changed = true;
                        }
                    }
                }
            }
        }
    }
}

impl UF {
    /// Create a term from a string representation (simplified).
    fn resolve_term(&mut self, s: &str) -> TermId {
        let s = s.trim();
        // Check if already created
        for (i, ft) in self.func_terms.iter().enumerate() {
            if format_term(ft) == s {
                return i;
            }
        }
        // Create new term
        if s.starts_with('(') && s.ends_with(')') {
            // Function application: (f arg1 arg2 ...)
            let inner = &s[1..s.len()-1].trim();
            let parts: Vec<&str> = inner.split_whitespace().collect();
            if parts.is_empty() {
                return self.new_const();
            }
            let name = parts[0].to_string();
            let args: Vec<TermId> = parts[1..].iter().map(|a| {
                let clean = a.trim_end_matches(')');
                self.resolve_term(clean)
            }).collect();
            self.new_app(&name, args)
        } else {
            // Constant
            self.new_const()
        }
    }
}

impl TheorySolver for UF {
    fn push(&mut self) {
        self.stack.push(self.assertions.len());
        self.stack.push(self.disequalities.len());
    }

    fn pop(&mut self) {
        // Restore assertion/disequality counts
        let diseq_len = self.stack.pop().unwrap();
        let assert_len = self.stack.pop().unwrap();
        self.assertions.truncate(assert_len);
        self.disequalities.truncate(diseq_len);
        // Reset union-find (simplified: full rebuild)
        for i in 0..self.next_term {
            self.parent[i] = i;
            self.rank[i] = 0;
        }
        let assertions = self.assertions.clone();
        for &(lhs, rhs) in &assertions {
            self.union(lhs, rhs);
        }
        self.propagate_congruence();
    }

    fn set_atoms(&mut self, atoms: &[Box<dyn TheoryAtom>]) {
        for atom in atoms {
            let label = atom.label();
            // Parse "t1 = t2" from label
            if let Some(eq_pos) = label.find(" = ") {
                let lhs_str = &label[..eq_pos];
                let rhs_str = &label[eq_pos + 3..];
                let lhs = self.resolve_term(lhs_str);
                let rhs = self.resolve_term(rhs_str);
                while self.atom_terms.len() <= atom.id() {
                    self.atom_terms.push((0, 0));
                }
                self.atom_terms[atom.id()] = (lhs, rhs);
            }
        }
    }

    fn assert(&mut self, lit: TheoryLit) -> bool {
        if lit.atom_id < self.atom_terms.len() {
            let (lhs, rhs) = self.atom_terms[lit.atom_id];
            if lit.sign {
                self.assertions.push((lhs, rhs));
                self.union(lhs, rhs);
            } else {
                self.disequalities.push((lhs, rhs));
            }
        }
        true
    }

    fn check(&self) -> TheoryCheckResult {
        let mut uf = self.clone();
        let assertions: Vec<(usize, usize)> = self.assertions.clone();
        let disequalities: Vec<(usize, usize)> = self.disequalities.clone();
        for &(lhs, rhs) in &assertions {
            uf.union(lhs, rhs);
        }
        uf.propagate_congruence();
        for &(lhs, rhs) in &disequalities {
            if uf.find(lhs) == uf.find(rhs) {
                return TheoryCheckResult::Conflict;
            }
        }
        TheoryCheckResult::Consistent
    }

    fn propagate(&self) -> Vec<TheoryLit> {
        vec![] // Basic UF: no propagation beyond congruence
    }

    fn explain_conflict(&self) -> Vec<TheoryLit> {
        vec![] // TODO: implement conflict explanation
    }

    fn explain_propagation(&self, _lit: TheoryLit) -> Vec<TheoryLit> {
        vec![]
    }

    fn reset(&mut self) {
        self.parent.clear();
        self.rank.clear();
        self.find_cache.clear();
        self.func_terms.clear();
        self.parents.clear();
        self.assertions.clear();
        self.disequalities.clear();
        self.next_term = 0;
        self.stack.clear();
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_find() {
        let mut uf = UF::new();
        let a = uf.new_const();
        let b = uf.new_const();
        let c = uf.new_const();
        assert_ne!(uf.find(a), uf.find(b));
        uf.union(a, b);
        assert_eq!(uf.find(a), uf.find(b));
        uf.union(b, c);
        assert_eq!(uf.find(a), uf.find(c));
    }

    #[test]
    fn test_congruence() {
        let mut uf = UF::new();
        let a = uf.new_const();
        let b = uf.new_const();
        // f(a) and f(b)
        let fa = uf.new_app("f", vec![a]);
        let fb = uf.new_app("f", vec![b]);
        assert_ne!(uf.find(fa), uf.find(fb));
        // a = b → f(a) = f(b)
        uf.union(a, b);
        uf.propagate_congruence();
        assert_eq!(uf.find(fa), uf.find(fb));
    }
}
