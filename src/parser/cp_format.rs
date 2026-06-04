//! Simple CP problem format parser.
//!
//! Format:
//! ```text
//! # variable declarations
//! var x 0 10
//! var y 0 10
//! # constraints
//! eq x y
//! neq x y
//! lt x y
//! allDiff x y z
//! ```

use crate::cp::{CpConstraint, CpProblem, CmpRel};

use super::ParseResult;

/// Simple CP format parser.
pub struct CpFormatParser;

impl CpFormatParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, input: &str) -> ParseResult<CpProblem> {
        let mut var_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut var_domains: Vec<(String, std::ops::RangeInclusive<i64>)> = Vec::new();
        let mut constraints: Vec<CpConstraint> = Vec::new();

        for (line_no, raw) in input.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "var" => {
                    if parts.len() < 4 {
                        return Err(format!("line {}: var needs name, min, max", line_no + 1));
                    }
                    let name = parts[1].to_string();
                    let min: i64 = parts[2].parse().map_err(|_| format!("line {}: bad min", line_no + 1))?;
                    let max: i64 = parts[3].parse().map_err(|_| format!("line {}: bad max", line_no + 1))?;
                    let id = var_domains.len();
                    var_map.insert(name.clone(), id);
                    var_domains.push((name, min..=max));
                }
                "eq" => {
                    if parts.len() < 3 { return Err("eq needs 2 vars".into()); }
                    let x = get_var(&var_map, parts[1], line_no)?;
                    let y = get_var(&var_map, parts[2], line_no)?;
                    constraints.push(CpConstraint::Eq(x, y));
                }
                "neq" => {
                    if parts.len() < 3 { return Err("neq needs 2 vars".into()); }
                    let x = get_var(&var_map, parts[1], line_no)?;
                    let y = get_var(&var_map, parts[2], line_no)?;
                    constraints.push(CpConstraint::Neq(x, y));
                }
                "lt" => {
                    if parts.len() < 3 { return Err("lt needs 2 vars".into()); }
                    let x = get_var(&var_map, parts[1], line_no)?;
                    let y = get_var(&var_map, parts[2], line_no)?;
                    constraints.push(CpConstraint::Lt(x, y));
                }
                "le" => {
                    if parts.len() < 4 { return Err("le needs 2 vars + c".into()); }
                    let x = get_var(&var_map, parts[1], line_no)?;
                    let y = get_var(&var_map, parts[2], line_no)?;
                    let c: i64 = parts[3].parse().map_err(|_| "bad c")?;
                    constraints.push(CpConstraint::Le(x, y, c));
                }
                "linear" => {
                    // linear coeff1 var1 coeff2 var2 ... rhs le/eq/ge
                    // We need at least: linear coeff var rhs rel
                    // Parse pairs of (coeff, var) then rhs then rel
                    if parts.len() < 5 { return Err("linear: need at least coeff var rhs rel".into()); }
                    let mut idx = 1;
                    let mut terms = Vec::new();
                    while idx + 2 < parts.len() - 2 {
                        let coeff: i64 = parts[idx].parse().map_err(|_| "bad coeff")?;
                        let var = get_var(&var_map, parts[idx + 1], line_no)?;
                        terms.push((coeff, var));
                        idx += 2;
                    }
                    if idx >= parts.len() - 2 {
                        return Err("linear: too few terms".into());
                    }
                    let rhs: i64 = parts[idx].parse().map_err(|_| "bad rhs")?;
                    let rel = match parts[idx + 1] {
                        "le" => CmpRel::Le,
                        "eq" => CmpRel::Eq,
                        "ge" => CmpRel::Ge,
                        _ => return Err("linear: expected le/eq/ge".into()),
                    };
                    constraints.push(CpConstraint::Linear(terms, rhs, rel));
                }
                "allDiff" | "alldifferent" => {
                    if parts.len() < 3 { return Err("allDiff needs ≥ 2 vars".into()); }
                    let vars: Vec<usize> = parts[1..].iter()
                        .map(|&s| get_var(&var_map, s, line_no))
                        .collect::<Result<Vec<_>, _>>()?;
                    constraints.push(CpConstraint::AllDifferent(vars));
                }
                _ => {
                    return Err(format!("line {}: unknown keyword '{}'", line_no + 1, parts[0]));
                }
            }
        }

        Ok(CpProblem {
            var_domains,
            constraints,
        })
    }
}

fn get_var(map: &std::collections::HashMap<String, usize>, name: &str, line_no: usize) -> ParseResult<usize> {
    map.get(name).copied().ok_or_else(|| {
        format!("line {}: unknown variable '{}'", line_no + 1, name)
    })
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_cp_parse() {
        let input = "var x 0 10\nvar y 0 10\neq x y\n";
        let problem = CpFormatParser::new().parse(input).unwrap();
        assert_eq!(problem.var_domains.len(), 2);
        assert_eq!(problem.constraints.len(), 1);
    }

    #[test]
    fn test_all_diff_parse() {
        let input = "var x 0 10\nvar y 0 10\nvar z 0 10\nallDiff x y z\n";
        let problem = CpFormatParser::new().parse(input).unwrap();
        assert_eq!(problem.constraints.len(), 1);
    }
}
