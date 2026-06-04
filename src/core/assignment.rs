use crate::core::{Value, Var};

/// An assignment maps variables to values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Assignment {
    values: Vec<(Var, Value)>,
}

impl Assignment {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Add or replace an assignment.
    pub fn set(&mut self, var: Var, value: Value) {
        if let Some(pair) = self.values.iter_mut().find(|(v, _)| *v == var) {
            pair.1 = value;
        } else {
            self.values.push((var, value));
        }
    }

    /// Get the value assigned to a variable, if any.
    pub fn get(&self, var: &Var) -> Option<&Value> {
        self.values
            .iter()
            .find(|(v, _)| v == var)
            .map(|(_, val)| val)
    }

    /// Iterate over all assignments.
    pub fn iter(&self) -> impl Iterator<Item = &(Var, Value)> {
        self.values.iter()
    }
}

impl std::fmt::Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pairs: Vec<String> = self
            .values
            .iter()
            .map(|(v, val)| format!("{} -> {}", v, val))
            .collect();
        write!(f, "[{}]", pairs.join(", "))
    }
}
