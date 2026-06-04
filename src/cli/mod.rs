//! Command-line interface definitions and helpers.

use clap::{Parser, Subcommand};

/// dsuni — a pure-Rust modular constraint solver.
#[derive(Parser, Debug)]
#[command(name = "dsuni", version, about)]
pub struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Solve a SAT problem (DIMACS CNF)
    Sat {
        /// Path to the input file
        file: String,
    },

    /// Solve an SMT problem (SMT-LIB format)
    Smt {
        /// Path to the input file
        file: String,
    },

    /// Solve a Linear Programming problem (LP format)
    Lp {
        /// Path to the input file
        file: String,
    },

    /// Solve a Mixed Integer Linear Programming problem (MPS/LP format)
    Milp {
        /// Path to the input file
        file: String,
    },

    /// Solve a Constraint Programming problem (MiniZinc subset)
    Cp {
        /// Path to the input file
        file: String,
    },

    /// Solve a Mixed Integer NonLinear Programming problem
    Minlp {
        /// Path to the input file
        file: String,
    },
}
