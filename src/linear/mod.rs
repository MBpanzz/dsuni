//! Linear arithmetic core — shared infrastructure for LP, MILP, and SMT-LRA/LIA.
//!
//! Provides exact rational arithmetic, linear expressions, constraints,
//! variable bounds, and a normalisation pipeline.

mod bound;
mod constraint;
mod expr;
mod normalize;
mod rational;
mod system;

pub use bound::*;
pub use constraint::*;
pub use expr::*;
pub use normalize::*;
pub use rational::*;
pub use system::*;

/// Re-export core variable type for convenience.
pub use crate::core::Var;
