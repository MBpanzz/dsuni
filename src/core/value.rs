use crate::core::Sort;

/// A concrete value assigned to a variable.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Real(f64),
}

impl Value {
    /// Return the sort of this value.
    pub fn sort(&self) -> Sort {
        match self {
            Value::Bool(_) => Sort::Bool,
            Value::Int(_) => Sort::Int,
            Value::Real(_) => Sort::Real,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Real(r) => write!(f, "{}", r),
        }
    }
}
