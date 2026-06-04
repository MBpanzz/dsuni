// dsuni CLI — pure-Rust modular constraint solver
use std::fs;
use std::path::Path;

use clap::Parser;
use dsuni::cli::{Cli, Commands};
use dsuni::core::Config;
use dsuni::cp::solve_cp;
use dsuni::lp::solve_lp;
use dsuni::minlp::solve_minlp;
use dsuni::milp::solve_milp;
use dsuni::parser::dimacs::DimacsParser;
use dsuni::parser::cp_format::CpFormatParser;
use dsuni::parser::lp_format::LpFormatParser;
use dsuni::parser::smtlib::SmtLibParser;
use dsuni::sat::solve_sat;
use dsuni::smt::solve_smt;

fn main() {
    let cli = Cli::parse();

    let config = Config::new().with_verbosity(if cli.verbose { 2 } else { 1 });

    match &cli.command {
        Commands::Sat { file } => {
            let input = read_file(file);
            let problem = DimacsParser::new().parse(&input).unwrap_or_else(|e| {
                eprintln!("DIMACS parse error: {e}");
                std::process::exit(1);
            });
            let result = solve_sat(&problem.clauses, &config);
            println!("{}", result);
        }
        Commands::Lp { file } => {
            let input = read_file(file);
            let problem = LpFormatParser::new().parse(&input).unwrap_or_else(|e| {
                eprintln!("LP parse error: {e}");
                std::process::exit(1);
            });
            let result = solve_lp(&problem.system, &config);
            println!("{}", result);
        }
        Commands::Smt { file } => {
            let input = read_file(file);
            let mut parser = SmtLibParser::new();
            let formula = parser.parse(&input).unwrap_or_else(|e| {
                eprintln!("SMT-LIB parse error: {e}");
                std::process::exit(1);
            });
            let result = solve_smt(&formula, &config);
            println!("{}", result);
        }
        Commands::Milp { file } => {
            let input = read_file(file);
            let problem = LpFormatParser::new().parse(&input).unwrap_or_else(|e| {
                eprintln!("LP/MILP parse error: {e}");
                std::process::exit(1);
            });
            let result = solve_milp(&problem.system, &config);
            println!("{}", result);
        }
        Commands::Cp { file } => {
            let input = read_file(file);
            let problem = CpFormatParser::new().parse(&input).unwrap_or_else(|e| {
                eprintln!("CP parse error: {e}");
                std::process::exit(1);
            });
            let result = solve_cp(&problem, &config);
            println!("{}", result);
        }
        Commands::Minlp { file } => {
            let input = read_file(file);
            // Simple MINLP format: each line "var <name> <lb> <ub> <int>"
            // or "bilinear <coeff> <i> <j>"
            // or "square <coeff> <i>"
            // or "linear <coeff> <i>"
            // or "le <rhs>" / "eq <rhs>"
            let mut problem = dsuni::minlp::MinlpProblem {
                vars: Vec::new(),
                constraints: Vec::new(),
            };
            let mut current_terms: Vec<dsuni::minlp::MinlpTerm> = Vec::new();
            for (line_no, raw) in input.lines().enumerate() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() { continue; }
                match parts[0] {
                    "var" if parts.len() >= 4 => {
                        let name = parts[1].to_string();
                        let lb: f64 = parts[2].parse().map_err(|_| "bad lb").unwrap_or(0.0);
                        let ub: f64 = parts[3].parse().map_err(|_| "bad ub").unwrap_or(0.0);
                        let is_int = parts.get(4).map_or(false, |&s| s == "int");
                        problem.vars.push(dsuni::minlp::MinlpVar { name, lb, ub, is_integer: is_int });
                    }
                    "linear" if parts.len() >= 3 => {
                        let coeff: f64 = parts[1].parse().map_err(|_| "bad coeff").unwrap_or(0.0);
                        let idx: usize = parts[2].parse().map_err(|_| "bad idx").unwrap_or(0);
                        current_terms.push(dsuni::minlp::MinlpTerm::Linear(coeff, idx));
                    }
                    "bilinear" if parts.len() >= 4 => {
                        let coeff: f64 = parts[1].parse().unwrap_or(0.0);
                        let i: usize = parts[2].parse().unwrap_or(0);
                        let j: usize = parts[3].parse().unwrap_or(0);
                        current_terms.push(dsuni::minlp::MinlpTerm::Bilinear(coeff, i, j));
                    }
                    "square" if parts.len() >= 3 => {
                        let coeff: f64 = parts[1].parse().unwrap_or(0.0);
                        let idx: usize = parts[2].parse().unwrap_or(0);
                        current_terms.push(dsuni::minlp::MinlpTerm::Square(coeff, idx));
                    }
                    "le" | "eq" if parts.len() >= 2 => {
                        let rhs: f64 = parts[1].parse().unwrap_or(0.0);
                        let is_eq = parts[0] == "eq";
                        let terms = std::mem::take(&mut current_terms);
                        problem.constraints.push(dsuni::minlp::MinlpConstraint { terms, rhs, is_eq });
                    }
                    _ => {
                        eprintln!("MINLP parse error: line {}: '{}'", line_no + 1, line);
                        std::process::exit(1);
                    }
                }
            }
            // Flush remaining terms
            if !current_terms.is_empty() {
                problem.constraints.push(dsuni::minlp::MinlpConstraint {
                    terms: current_terms,
                    rhs: 0.0,
                    is_eq: false,
                });
            }
            let result = solve_minlp(&problem, &config);
            println!("{}", result);
        }
    }
}

fn read_file(path: &str) -> String {
    let p = Path::new(path);
    if !p.exists() {
        eprintln!("File not found: {path}");
        std::process::exit(1);
    }
    fs::read_to_string(p).unwrap_or_else(|e| {
        eprintln!("Error reading {path}: {e}");
        std::process::exit(1);
    })
}
