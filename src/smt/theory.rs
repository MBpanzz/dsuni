/// The result of a theory check.
#[derive(Debug, Clone, PartialEq)]
pub enum TheoryCheckResult {
    /// The current assignment is consistent with the theory.
    Consistent,
    /// The current assignment is inconsistent.
    Conflict,
    /// Unknown (incomplete theory).
    Unknown,
}

/// A theory atom: a basic formula in a theory (e.g., x = y, x ≤ 5).
pub trait TheoryAtom: std::fmt::Debug {
    /// Unique identifier for this atom.
    fn id(&self) -> usize;
    /// A human-readable label.
    fn label(&self) -> String;
    /// Clone into a boxed trait object.
    fn clone_box(&self) -> Box<dyn TheoryAtom>;
}

// Allow cloning Vec<Box<dyn TheoryAtom>>
impl Clone for Box<dyn TheoryAtom> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A theory propagation: implication from the theory solver.
#[derive(Debug, Clone)]
pub struct TheoryPropagation {
    pub lit: TheoryLit,
    pub explanation: Vec<TheoryLit>,
}


use crate::smt::TheoryLit;

/// The trait that every theory solver must implement.
///
/// This follows the lazy SMT (CDCL(T)) interface:
/// - `assert` assigns a theory literal
/// - `check` checks consistency of current assignment
/// - `propagate` returns implied literals
/// - `explain` returns the reason for a conflict or propagation
/// - `push`/`pop` for scoped assertions (backtracking)
pub trait TheorySolver: std::fmt::Debug {
    /// Push a new assertion scope.
    fn push(&mut self);

    /// Pop the last scope.
    fn pop(&mut self);

    /// Assert a theory literal. Returns true if consistent.
    fn assert(&mut self, lit: TheoryLit) -> bool;

    /// Check consistency of the current assignment.
    fn check(&self) -> TheoryCheckResult;

    /// Propagate: return literals that are implied by the current assignment.
    fn propagate(&self) -> Vec<TheoryLit>;

    /// Explain a conflict: return the set of theory literals causing the conflict.
    fn explain_conflict(&self) -> Vec<TheoryLit>;

    /// Explain a propagation.
    fn explain_propagation(&self, lit: TheoryLit) -> Vec<TheoryLit>;

    /// Set the list of theory atoms (with their labels/definitions).
    fn set_atoms(&mut self, atoms: &[Box<dyn TheoryAtom>]);

    /// Reset the solver.
    fn reset(&mut self);
}
