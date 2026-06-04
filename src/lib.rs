//! dsuni — a modular constraint solver in pure Rust.
//!
//! This library provides the shared infrastructure for all solver modules:
//!   - `core`     : fundamental types (Var, Term, Constraint, Model, ...)
//!   - `parser`   : input format parsers (DIMACS, SMT-LIB, LP, ...)
//!   - `normalize`: term normalisation and type checking
//!   - `cli`      : command-line interface helpers

pub mod cli;
pub mod core;
pub mod cp;
pub mod linear;
pub mod lp;
pub mod milp;
pub mod minlp;
pub mod normalize;
pub mod parser;
pub mod sat;
pub mod smt;

#[cfg(feature = "python")]
mod python;
