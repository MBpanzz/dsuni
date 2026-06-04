//! DIMACS CNF parser (SAT).
//!
//! The DIMACS CNF format:
//! ```text
//!   c comment line
//!   p cnf <variables> <clauses>
//!   1 -2 3 0
//!   -1 0
//! ```
//!
//! Lines starting with `c` are comments.
//! The problem line `p cnf V C` gives variable and clause counts.
//! Each clause is a sequence of non-zero integers terminated by `0`.
//! Positive = variable, negative = ¬variable.

use super::ParseResult;

/// A parsed DIMACS problem.
#[derive(Debug, Clone)]
pub struct DimacsProblem {
    /// Number of variables declared.
    pub num_vars: usize,
    /// Number of clauses declared.
    pub num_clauses: usize,
    /// List of clauses, each as DIMACS integers (±1-indexed variable).
    pub clauses: Vec<Vec<i32>>,
}

/// DIMACS CNF parser.
pub struct DimacsParser;

impl DimacsParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a complete DIMACS CNF input.
    pub fn parse(&self, input: &str) -> ParseResult<DimacsProblem> {
        let mut num_vars = 0;
        let mut num_clauses = 0;
        let mut clauses: Vec<Vec<i32>> = Vec::new();
        let mut current_clause: Vec<i32> = Vec::new();
        let mut found_problem = false;

        for (line_no, raw_line) in input.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            // Comment line
            if line.starts_with('c') {
                continue;
            }

            // Problem line: p cnf <vars> <clauses>
            if line.starts_with('p') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 || parts[1] != "cnf" {
                    return Err(format!(
                        "line {}: invalid problem line, expected 'p cnf <vars> <clauses>'",
                        line_no + 1
                    ));
                }
                num_vars = parts[2]
                    .parse()
                    .map_err(|_| format!("line {}: invalid variable count", line_no + 1))?;
                num_clauses = parts[3]
                    .parse()
                    .map_err(|_| format!("line {}: invalid clause count", line_no + 1))?;
                found_problem = true;

                // Flush any clause parsed before the problem line.
                if !current_clause.is_empty() {
                    if current_clause.last() != Some(&0) {
                        return Err(format!(
                            "line {}: clause before problem header not terminated",
                            line_no + 1
                        ));
                    }
                    current_clause.pop(); // remove the trailing 0
                    if !current_clause.is_empty() {
                        clauses.push(current_clause.clone());
                    }
                    current_clause.clear();
                }
                continue;
            }

            if !found_problem {
                return Err(format!(
                    "line {}: data before 'p cnf' header",
                    line_no + 1
                ));
            }

            // Parse clause literals.
            for token in line.split_whitespace() {
                let val: i32 = token.parse().map_err(|_| {
                    format!("line {}: invalid literal '{}'", line_no + 1, token)
                })?;

                if val == 0 {
                    // End of clause — push even if empty (empty clause = UNSAT).
                    clauses.push(current_clause.clone());
                    current_clause.clear();
                } else {
                    current_clause.push(val);
                }
            }
        }

        // Flush any remaining clause.
        if !current_clause.is_empty() {
            clauses.push(current_clause);
        }

        if !found_problem {
            return Err("missing 'p cnf' header".into());
        }

        Ok(DimacsProblem {
            num_vars,
            num_clauses,
            clauses,
        })
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_cnf() {
        let input = "c simple example\np cnf 3 2\n1 -2 0\n-1 3 0\n";
        let prob = DimacsParser::new().parse(input).unwrap();
        assert_eq!(prob.num_vars, 3);
        assert_eq!(prob.num_clauses, 2);
        assert_eq!(prob.clauses, vec![vec![1, -2], vec![-1, 3]]);
    }

    #[test]
    fn test_unit_clause() {
        let input = "p cnf 1 1\n1 0\n";
        let prob = DimacsParser::new().parse(input).unwrap();
        assert_eq!(prob.clauses, vec![vec![1]]);
    }

    #[test]
    fn test_empty_clause() {
        let input = "p cnf 1 1\n0\n";
        let prob = DimacsParser::new().parse(input).unwrap();
        assert_eq!(prob.clauses, vec![vec![]] as Vec<Vec<i32>>);
    }

    #[test]
    fn test_multiline_clause() {
        let input = "p cnf 3 1\n1 2\n3 0\n";
        let prob = DimacsParser::new().parse(input).unwrap();
        assert_eq!(prob.clauses, vec![vec![1, 2, 3]]);
    }

    #[test]
    fn test_data_before_header_error() {
        let result = DimacsParser::new().parse("1 0\n");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("before"));
    }

    #[test]
    fn test_data_before_header() {
        let result = DimacsParser::new().parse("1 0\np cnf 1 1\n");
        assert!(result.is_err());
    }
}
