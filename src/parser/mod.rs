//! Input format parsers.
//!
//! Each sub-module handles a specific format:
//!   - `dimacs`   : CNF / DIMACS (SAT)
//!   - `smtlib`   : SMT-LIB (SMT)
//!   - `lp`       : LP format (LP / MILP)
//!   - `mps`      : MPS format (MILP)
//!   - `mzn`      : MiniZinc subset (CP)
//!
//! At this stage only stub modules exist.

pub mod cp_format;
pub mod dimacs;
pub mod lp_format;
pub mod mps;
pub mod mzn;
pub mod smtlib;

/// Common parse-result type used by all parsers.
pub type ParseResult<T> = Result<T, String>;
