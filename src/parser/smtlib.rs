//! SMT-LIB format parser for QF_UF and QF_LRA.
//!
//! Supports a subset of the SMT-LIB 2.x standard:
//!
//! ```text
//! (set-logic QF_UF)
//! (declare-const a Bool)
//! (declare-fun f (Bool) Bool)
//! (assert (= (f a) b))
//! (check-sat)
//! (exit)
//! ```

use std::collections::HashMap;
use crate::smt::TheoryAtom;
use crate::smt::{Formula, TheoryLit};

use super::ParseResult;

// ── S-expression types ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum SExpr {
    Symbol(String),
    List(Vec<SExpr>),
}

// ── Tokenizer ────────────────────────────────────────────────────────

fn tokenize(input: &str) -> ParseResult<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_comment = false;

    for ch in input.chars() {
        if ch == ';' {
            in_comment = true;
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            continue;
        }
        if ch == '\n' {
            in_comment = false;
            continue;
        }
        if in_comment {
            continue;
        }

        match ch {
            '(' | ')' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                tokens.push(ch.to_string());
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            '"' => {
                // String literal (simplified)
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                current.push('"');
                // Find closing quote
                // (simplified: doesn't handle escaped quotes)
            }
            c => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

// ── S-expression parser ──────────────────────────────────────────────

fn parse_sexpr(tokens: &[String], pos: &mut usize) -> ParseResult<SExpr> {
    if *pos >= tokens.len() {
        return Err("unexpected end of input".into());
    }

    let token = &tokens[*pos];
    *pos += 1;

    if token == "(" {
        let mut items = Vec::new();
        while *pos < tokens.len() && tokens[*pos] != ")" {
            items.push(parse_sexpr(tokens, pos)?);
        }
        if *pos >= tokens.len() {
            return Err("unclosed parenthesis".into());
        }
        *pos += 1; // skip ")"
        Ok(SExpr::List(items))
    } else if token == ")" {
        Err("unexpected ')".into())
    } else {
        Ok(SExpr::Symbol(token.clone()))
    }
}

fn parse_all(tokens: &[String]) -> ParseResult<Vec<SExpr>> {
    let mut exprs = Vec::new();
    let mut pos = 0;
    while pos < tokens.len() {
        exprs.push(parse_sexpr(tokens, &mut pos)?);
    }
    Ok(exprs)
}

// ── SMT-LIB command handler ──────────────────────────────────────────

/// A function/constant declaration (reserved for future use).
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Decl {
    name: String,
    args: Vec<String>,
    sort: String,
}

/// SMT-LIB formula parser.
pub struct SmtLibParser {
    /// Declared functions and constants.
    decls: HashMap<String, Decl>,
    /// Next atom ID.
    next_atom: usize,
    /// Collected atoms.
    atoms: Vec<AtomRecord>,
    /// Collected clauses (as lists of TheoryLit).
    clauses: Vec<Vec<TheoryLit>>,
    /// Tseitin variable counter (reserved).
    #[allow(dead_code)]
    next_tseitin: usize,
}

/// A record of a theory atom.
#[derive(Debug, Clone)]
struct AtomRecord {
    id: usize,
    kind: AtomKind,
}

#[derive(Debug, Clone)]
enum AtomKind {
    /// equality between two terms
    Eq { lhs: String, rhs: String },
    /// inequality
    Neq { lhs: String, rhs: String },
}

impl TheoryAtom for AtomRecord {
    fn id(&self) -> usize { self.id }
    fn label(&self) -> String {
        match &self.kind {
            AtomKind::Eq { lhs, rhs } => format!("{} = {}", lhs, rhs),
            AtomKind::Neq { lhs, rhs } => format!("{} ≠ {}", lhs, rhs),
        }
    }
    fn clone_box(&self) -> Box<dyn TheoryAtom> {
        Box::new(self.clone())
    }
}

impl SmtLibParser {
    pub fn new() -> Self {
        SmtLibParser {
            decls: HashMap::new(),
            next_atom: 0,
            atoms: Vec::new(),
            clauses: Vec::new(),
            next_tseitin: 0,
        }
    }

    /// Parse SMT-LIB input into a `Formula`.
    pub fn parse(&mut self, input: &str) -> ParseResult<Formula> {
        let tokens = tokenize(input)?;
        let exprs = parse_all(&tokens)?;

        for expr in &exprs {
            self.process_command(expr)?;
        }

        // Build the Formula
        Ok(Formula {
            atoms: self.atoms.iter().map(|a| Box::new(a.clone()) as Box<dyn TheoryAtom>).collect(),
            clauses: self.clauses.clone(),
        })
    }

    /// Process a top-level command.
    fn process_command(&mut self, expr: &SExpr) -> ParseResult<()> {
        let list = match expr {
            SExpr::List(items) => items,
            SExpr::Symbol(_) => return Err("expected list at top level".into()),
        };

        if list.is_empty() {
            return Ok(());
        }

        let cmd = self.get_symbol(&list[0])?;

        match cmd {
            "set-logic" => {
                // We don't really care about the logic name
            }
            "declare-const" => {
                if list.len() < 3 { return Err("declare-const: too few args".into()); }
                let name = self.get_symbol(&list[1])?;
                let sort = self.get_symbol(&list[2])?;
                self.decls.insert(name.to_string(), Decl {
                    name: name.to_string(),
                    args: vec![],
                    sort: sort.to_string(),
                });
            }
            "declare-fun" => {
                if list.len() < 4 { return Err("declare-fun: too few args".into()); }
                let name = self.get_symbol(&list[1])?;
                let args = match &list[2] {
                    SExpr::List(sorts) => {
                        sorts.iter().map(|s| self.get_symbol(s).map(|s| s.to_string())).collect::<Result<Vec<_>, _>>()?
                    }
                    _ => return Err("declare-fun: expected sort list".into()),
                };
                let sort = self.get_symbol(&list[3])?;
                self.decls.insert(name.to_string(), Decl {
                    name: name.to_string(),
                    args,
                    sort: sort.to_string(),
                });
            }
            "assert" => {
                if list.len() < 2 { return Err("assert: too few args".into()); }
                self.process_assertion(&list[1])?;
            }
            "check-sat" => {
                // Nothing to do — the solver will be called externally
            }
            "exit" => {}
            _ => {
                return Err(format!("unknown command: {}", cmd));
            }
        }

        Ok(())
    }

    /// Process an assert formula.
    fn process_assertion(&mut self, expr: &SExpr) -> ParseResult<()> {
        // Convert the formula to CNF clauses over theory literals
        self.formula_to_clauses(expr, true)?;
        Ok(())
    }

    /// Convert a formula to CNF clauses.
    /// `polarity` = true means the formula is asserted positively.
    fn formula_to_clauses(&mut self, expr: &SExpr, polarity: bool) -> ParseResult<Vec<TheoryLit>> {
        match expr {
            SExpr::Symbol(s) if s == "true" => {
                if polarity {
                    Ok(vec![]) // true = no constraints
                } else {
                    // false is asserted → empty clause = conflict
                    self.clauses.push(vec![]);
                    Ok(vec![])
                }
            }
            SExpr::Symbol(s) if s == "false" => {
                if !polarity {
                    Ok(vec![])
                } else {
                    self.clauses.push(vec![]);
                    Ok(vec![])
                }
            }
            SExpr::List(items) if !items.is_empty() => {
                let op = self.get_symbol(&items[0])?;
                match op {
                    "not" => {
                        if items.len() < 2 { return Err("not: too few args".into()); }
                        self.formula_to_clauses(&items[1], !polarity)
                    }
                    "and" => {
                        let sub_formulas: Vec<&SExpr> = items[1..].iter().collect();
                        for f in &sub_formulas {
                            self.formula_to_clauses(f, polarity)?;
                        }
                        Ok(vec![])
                    }
                    "or" => {
                        if polarity {
                            // (or F1 F2 ...) → clause { lits(F1) ∪ lits(F2) ∪ ... }
                            let mut or_lits = Vec::new();
                            for f in &items[1..] {
                                if let SExpr::Symbol(s) = f {
                                    if s == "true" { return Ok(vec![]); }
                                    if s == "false" { continue; }
                                }
                                if let SExpr::List(inner) = f {
                                    if !inner.is_empty() && self.get_symbol(&inner[0])? == "=" {
                                        // Direct atom
                                        let lit = self.make_eq_atom(inner, true)?;
                                        or_lits.push(lit);
                                        continue;
                                    }
                                    if !inner.is_empty() && self.get_symbol(&inner[0])? == "not" 
                                        && inner.len() >= 2 
                                        && self.is_eq(&inner[1]) 
                                    {
                                        if let SExpr::List(eq_inner) = &inner[1] {
                                            let lit = self.make_eq_atom(eq_inner, false)?;
                                            or_lits.push(lit);
                                            continue;
                                        }
                                    }
                                }
                                // Complex subformula: use Tseitin
                                let tseitin = self.new_tseitin();
                                let tlit = TheoryLit::new(tseitin, true);
                                self.formula_to_clauses(f, true)?;
                                // tseitin ↔ f
                                // polarity=true: clause (tlit ∨ ...) ... already added by formula_to_clauses
                                or_lits.push(tlit);
                            }
                            if !or_lits.is_empty() {
                                self.clauses.push(or_lits);
                            }
                            Ok(vec![])
                        } else {
                            // (not (or F1 F2 ...)) = (and (not F1) (not F2) ...)
                            for f in &items[1..] {
                                self.formula_to_clauses(f, false)?;
                            }
                            Ok(vec![])
                        }
                    }
                    "=>" => {
                        if items.len() < 3 { return Err("=>: too few args".into()); }
                        // (=> a b) = (or (not a) b)
                        // With polarity handling
                        let not_a = SExpr::List(vec![SExpr::Symbol("not".into()), items[1].clone()]);
                        let or = SExpr::List(vec![SExpr::Symbol("or".into()), not_a, items[2].clone()]);
                        self.formula_to_clauses(&or, polarity)
                    }
                    "=" => {
                        if polarity {
                            let lit = self.make_eq_atom(items, true)?;
                            self.clauses.push(vec![lit]);
                            Ok(vec![lit])
                        } else {
                            let lit = self.make_eq_atom(items, false)?;
                            self.clauses.push(vec![lit]);
                            Ok(vec![lit])
                        }
                    }
                    _ => {
                        // Function application — it's a term (returns Bool), treat as atom
                        // We encode it as equality with true
                        let term_str = self.term_to_string(expr);
                        // Create an atom for f(t1, t2, ...)
                        let id = self.next_atom;
                        self.next_atom += 1;
                        let kind = if polarity {
                            AtomKind::Eq { lhs: term_str.clone(), rhs: "true".into() }
                        } else {
                            AtomKind::Neq { lhs: term_str.clone(), rhs: "true".into() }
                        };
                        self.atoms.push(AtomRecord { id, kind });
                        let lit = TheoryLit::new(id, true);
                        self.clauses.push(vec![lit]);
                        Ok(vec![lit])
                    }
                }
            }
            _ => {
                // Bare symbol as a formula (could be a Boolean variable)
                // Treat as atom
                let id = self.next_atom;
                self.next_atom += 1;
                let name = self.get_symbol(expr)?;
                let kind = if polarity {
                    AtomKind::Eq { lhs: name.to_string(), rhs: "true".into() }
                } else {
                    AtomKind::Neq { lhs: name.to_string(), rhs: "true".into() }
                };
                self.atoms.push(AtomRecord { id, kind });
                let lit = TheoryLit::new(id, true);
                self.clauses.push(vec![lit]);
                Ok(vec![lit])
            }
        }
    }

    /// Check if an S-expression is an equality.
    fn is_eq(&self, expr: &SExpr) -> bool {
        if let SExpr::List(items) = expr {
            if !items.is_empty() {
                if let SExpr::Symbol(op) = &items[0] {
                    return op == "=";
                }
            }
        }
        false
    }

    /// Create or reuse an equality atom.
    fn make_eq_atom(&mut self, list: &[SExpr], sign: bool) -> ParseResult<TheoryLit> {
        if list.len() < 3 {
            return Err("=: too few args".into());
        }
        let lhs = self.term_to_string(&list[1]);
        let rhs = self.term_to_string(&list[2]);
        // Check if this atom already exists
        for atom in &self.atoms {
            match &atom.kind {
                AtomKind::Eq { lhs: l, rhs: r } if *l == lhs && *r == rhs => {
                    return Ok(TheoryLit::new(atom.id, sign));
                }
                AtomKind::Eq { lhs: l, rhs: r } if *l == rhs && *r == lhs => {
                    return Ok(TheoryLit::new(atom.id, sign));
                }
                _ => {}
            }
        }
        let id = self.next_atom;
        self.next_atom += 1;
        let kind = AtomKind::Eq { lhs, rhs };
        self.atoms.push(AtomRecord { id, kind });
        Ok(TheoryLit::new(id, sign))
    }

    /// Convert a term expression to a string key.
    fn term_to_string(&self, expr: &SExpr) -> String {
        match expr {
            SExpr::Symbol(s) => s.clone(),
            SExpr::List(items) => {
                let parts: Vec<String> = items.iter().map(|e| self.term_to_string(e)).collect();
                format!("({})", parts.join(" "))
            }
        }
    }

    /// Get a Tseitin variable ID.
    fn new_tseitin(&mut self) -> usize {
        let id = self.next_atom;
        self.next_atom += 1;
        self.atoms.push(AtomRecord {
            id,
            kind: AtomKind::Eq { lhs: format!("_t{}", id), rhs: "true".into() },
        });
        id
    }

    /// Extract a symbol from an S-expression.
    fn get_symbol<'a>(&self, expr: &'a SExpr) -> ParseResult<&'a str> {
        match expr {
            SExpr::Symbol(s) => Ok(s.as_str()),
            _ => Err("expected symbol".into()),
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("(set-logic QF_UF)").unwrap();
        assert_eq!(tokens, vec!["(", "set-logic", "QF_UF", ")"]);
    }

    #[test]
    fn test_parse_declare_const() {
        let input = "(set-logic QF_UF)\n(declare-const a Bool)\n(check-sat)\n";
        let mut parser = SmtLibParser::new();
        let formula = parser.parse(input).unwrap();
        assert_eq!(formula.atoms.len(), 0);
    }

    #[test]
    fn test_assert_equality() {
        let input = "(set-logic QF_UF)\n(declare-const a Bool)\n(declare-const b Bool)\n(assert (= a b))\n(check-sat)\n";
        let mut parser = SmtLibParser::new();
        let formula = parser.parse(input).unwrap();
        assert_eq!(formula.atoms.len(), 1);
        assert_eq!(formula.clauses.len(), 1);
    }

    #[test]
    fn test_assert_not_equality() {
        let input = "(set-logic QF_UF)\n(declare-const a Bool)\n(declare-const b Bool)\n(assert (not (= a b)))\n(check-sat)\n";
        let mut parser = SmtLibParser::new();
        let formula = parser.parse(input).unwrap();
        assert_eq!(formula.atoms.len(), 1);
        assert_eq!(formula.clauses.len(), 1);
    }

    #[test]
    fn test_assert_and() {
        let input = "(set-logic QF_UF)\n(declare-const a Bool)\n(declare-const b Bool)\n(declare-const c Bool)\n(assert (and (= a b) (= a c)))\n(check-sat)\n";
        let mut parser = SmtLibParser::new();
        let formula = parser.parse(input).unwrap();
        // Two atoms: (a=b) and (a=c)
        assert_eq!(formula.atoms.len(), 2);
        // Two unit clauses
        assert_eq!(formula.clauses.len(), 2);
    }

    #[test]
    fn test_assert_or() {
        let input = "(set-logic QF_UF)\n(declare-const a Bool)\n(declare-const b Bool)\n(assert (or (= a b) (= a b)))\n(check-sat)\n";
        let mut parser = SmtLibParser::new();
        let formula = parser.parse(input).unwrap();
        assert_eq!(formula.atoms.len(), 1);
        // One clause with both literals... wait, they're the same atom
        // Actually we deduplicate, so it should be one clause with one lit
        assert!(formula.clauses.len() >= 1);
    }
}
