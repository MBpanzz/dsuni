use crate::core::Assignment;

/// A full model produced by a successful solve.
///
/// Contains the assignments for all relevant variables.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub assignments: Assignment,
}

impl Model {
    pub fn new(assignments: Assignment) -> Self {
        Self { assignments }
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Model: {}", self.assignments)
    }
}
