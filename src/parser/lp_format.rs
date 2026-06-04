//! Minimal LP file format parser.
//!
//! Supports a simple text format:
//!
//! ```text
//! min -1*x0 + 2*x1
//! x0 + 2*x1 <= 10
//! x0 >= 0
//! x1 >= 0
//! ```
//!
//! - `min <expr>` or `max <expr>` sets the objective.
//! - `<expr> <= <num>`, `<expr> >= <num>`, `<expr> = <num>` are constraints.
//! - Variables are `x` followed by digits (`x0`, `x1`, …).
//! - Terms: `coeff*var` or `var`.

use crate::linear::{CmpOp, LinearConstraint, LinearExpr, LinearSystem, Objective, Rational, Var};
use super::ParseResult;

/// Parsed LP problem.
pub struct LpProblem {
    pub system: LinearSystem,
}

/// Minimal LP format parser.
pub struct LpFormatParser;

impl LpFormatParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse LP text into a `LinearSystem`.
    pub fn parse(&self, input: &str) -> ParseResult<LpProblem> {
        let mut sys = LinearSystem::new();
        let mut objective: Option<(bool, String)> = None;
        let mut constraints: Vec<(String, CmpOp, f64)> = Vec::new();
        let mut int_vars: Vec<String> = Vec::new();

        for (line_no, raw) in input.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('c') {
                continue;
            }
            if line == "end" {
                break;
            }

            if let Some(rest) = line.strip_prefix("min ") {
                objective = Some((true, rest.trim_end_matches(';').to_string()));
                continue;
            }
            if let Some(rest) = line.strip_prefix("max ") {
                objective = Some((false, rest.trim_end_matches(';').to_string()));
                continue;
            }

            if let Some(var_name) = line.strip_prefix("int ") {
                let name = var_name.trim().to_string();
                if !name.is_empty() {
                    int_vars.push(name);
                }
                continue;
            }

            if let Some(pos) = line.find("<=") {
                let e = line[..pos].trim().to_string();
                let r = line[pos + 2..].trim().trim_end_matches(';').trim();
                constraints.push((e, CmpOp::Le, r.parse::<f64>().map_err(|_| format!("bad number: {}", r))?));
                continue;
            }
            if let Some(pos) = line.find(">=") {
                let e = line[..pos].trim().to_string();
                let r = line[pos + 2..].trim().trim_end_matches(';').trim();
                constraints.push((e, CmpOp::Ge, r.parse::<f64>().map_err(|_| format!("bad number: {}", r))?));
                continue;
            }
            if let Some(pos) = line.find('=') {
                if !line.contains("<=") && !line.contains(">=") {
                    let e = line[..pos].trim().to_string();
                    let r = line[pos + 1..].trim().trim_end_matches(';').trim();
                    constraints.push((e, CmpOp::Eq, r.parse::<f64>().map_err(|_| format!("bad number: {}", r))?));
                    continue;
                }
            }

            return Err(format!("line {}: bad syntax: {}", line_no + 1, line));
        }

        // Parse objective
        if let Some((is_min, text)) = &objective {
            let expr = parse_expr(text, &mut sys)?;
            if *is_min {
                sys.set_objective(Objective::Minimise(expr));
            } else {
                sys.set_objective(Objective::Maximise(expr));
            }
        }

        // Apply integer declarations
        let all_vars: Vec<_> = sys.vars().to_vec();
        for int_name in &int_vars {
            for &v in &all_vars {
                if format!("x{}", v.id()) == *int_name {
                    sys.set_integer(v, true);
                }
            }
        }

        // Parse constraints
        for (expr_str, op, rhs_val) in &constraints {
            let mut expr = parse_expr(expr_str, &mut sys)?;
            expr = expr - LinearExpr::constant(rational(*rhs_val));
            let c = match op {
                CmpOp::Le => LinearConstraint::le(expr),
                CmpOp::Ge => LinearConstraint::ge(expr),
                CmpOp::Eq => LinearConstraint::eq(expr),
            };
            sys.add_constraint(c);
        }

        Ok(LpProblem { system: sys })
    }
}

/// Parse a linear expression like `-1*x0 + 2*x1` or `x0 - x1` or `-x0`.
fn parse_expr(text: &str, sys: &mut LinearSystem) -> ParseResult<LinearExpr> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(LinearExpr::ZERO);
    }

    // Tokenize: split on + and - keeping signs
    let mut terms: Vec<(i64, String)> = Vec::new();
    let mut current = String::new();
    let mut sign: i64 = 1;

    for ch in text.chars() {
        match ch {
            '+' => {
                if !current.is_empty() {
                    terms.push((sign, current.clone()));
                    current.clear();
                }
                sign = 1;
            }
            '-' => {
                if !current.is_empty() {
                    terms.push((sign, current.clone()));
                    current.clear();
                }
                sign = -1;
            }
            ' ' | '\t' => {
                if !current.is_empty() {
                    // space in middle of expression, treat as separator unless followed by +/-
                    // just ignore and keep accumulating
                    current.push(' ');
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        terms.push((sign, current.clone()));
    }

    if terms.is_empty() {
        return Ok(LinearExpr::ZERO);
    }

    let mut result = LinearExpr::ZERO;
    for (sign, term) in terms {
        let term = term.trim();
        if term.is_empty() {
            continue;
        }

        // Check for constant rational like "3/4"
        if let Some(slash) = term.find('/') {
            let num: f64 = term[..slash].trim().parse()
                .map_err(|_| format!("bad num in {}", term))?;
            let den: f64 = term[slash+1..].trim().parse()
                .map_err(|_| format!("bad den in {}", term))?;
            let val = rational(num / den) * Rational::from_i64(sign);
            result = result + LinearExpr::constant(val);
            continue;
        }

        // Check for "coeff*var" or just "var" or just a number
        if let Some(star) = term.find('*') {
            let coeff_str = term[..star].trim();
            let var_str = term[star+1..].trim();
            let coeff: f64 = coeff_str.parse()
                .map_err(|_| format!("bad coeff in {}", term))?;
            let v = get_var(var_str, sys);
            let expr = LinearExpr::from_var(v) * rational(coeff * sign as f64);
            result = result + expr;
        } else if term.contains(|c: char| c.is_alphabetic()) {
            // bare variable
            let v = get_var(term, sys);
            if sign == 1 {
                result = result + LinearExpr::from_var(v);
            } else {
                result = result - LinearExpr::from_var(v);
            }
        } else {
            // plain number
            let val: f64 = term.parse()
                .map_err(|_| format!("bad constant in {}", term))?;
            result = result + LinearExpr::constant(rational(val * sign as f64));
        }
    }

    Ok(result)
}

/// Get or create a variable by name in the system.
fn get_var(name: &str, sys: &mut LinearSystem) -> Var {
    let name = name.trim();
    // Check if already exists by searching var list
    for v in sys.vars() {
        if format!("x{}", v.id()) == name {
            return *v;
        }
    }
    sys.new_var()
}

fn rational(x: f64) -> Rational {
    if x == 0.0 { return Rational::ZERO; }
    if x.fract() == 0.0 { return Rational::from_i64(x as i64); }
    let scale = 1_000_000_000i64;
    Rational::new((x * scale as f64).round() as i64, scale)
}

// ── helpers on Rational ───────────────────────────────────────────────

trait RationalExt {
    fn from_i64(n: i64) -> Self;
}

impl RationalExt for Rational {
    fn from_i64(n: i64) -> Self {
        Rational::new(n, 1)
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_parse() {
        let input = "min -1*x0\nx0 <= 5\nx0 >= 0\n";
        let p = LpFormatParser::new().parse(input).unwrap();
        assert!(!p.system.objective().is_feasibility());
        assert_eq!(p.system.num_constraints(), 2);
        assert_eq!(p.system.num_vars(), 1);
    }

    #[test]
    fn test_parse_with_comments() {
        let input = "# test\nc comment\nmin -x0\nx0 <= 10\nend\n";
        let p = LpFormatParser::new().parse(input).unwrap();
        assert_eq!(p.system.num_constraints(), 1);
    }

    #[test]
    fn test_parse_two_vars() {
        let input = "max 3*x0 + 2*x1\nx0 + x1 <= 10\nx0 >= 0\nx1 >= 0\n";
        let p = LpFormatParser::new().parse(input).unwrap();
        assert_eq!(p.system.num_vars(), 2);
        assert_eq!(p.system.num_constraints(), 3);
    }
}
