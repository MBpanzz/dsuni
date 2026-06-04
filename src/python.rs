//! Python bindings for dsuni.
//!
//! Exposes the main solver APIs to Python via PyO3.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::core::{Config, SolverResult, LpResult, Value};
use crate::parser::dimacs::DimacsParser;
use crate::parser::lp_format::LpFormatParser;
use crate::parser::cp_format::CpFormatParser;

/// Python module: `dsuni`.
#[pymodule]
fn dsuni(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(solve_sat, m)?)?;
    m.add_function(wrap_pyfunction!(solve_lp_text, m)?)?;
    m.add_function(wrap_pyfunction!(solve_milp_text, m)?)?;
    m.add_function(wrap_pyfunction!(solve_cp_text, m)?)?;
    Ok(())
}

// ── result dict helpers ──────────────────────────────────────────────

fn model_to_py(assignments: &crate::core::Assignment) -> Py<PyAny> {
    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        for (var, val) in assignments.iter() {
            let key = format!("x{}", var.id());
            match val {
                Value::Bool(b) => { dict.set_item(key, *b).ok(); }
                Value::Int(i) => { dict.set_item(key, *i).ok(); }
                Value::Real(r) => { dict.set_item(key, *r).ok(); }
            }
        }
        dict.into_py(py)
    })
}

// ── SAT solver ───────────────────────────────────────────────────────

/// Solve a SAT problem from DIMACS CNF text.
/// Returns a dict with keys: status, model (if SAT).
#[pyfunction]
fn solve_sat(input: &str) -> PyResult<Py<PyAny>> {
    let parser = DimacsParser::new();
    let problem = parser.parse(input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
    let result = crate::sat::solve_sat(&problem.clauses, &Config::default());

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        match result {
            SolverResult::Sat(model) => {
                dict.set_item("status", "SAT")?;
                dict.set_item("model", model_to_py(&model.assignments))?;
            }
            SolverResult::Unsat => {
                dict.set_item("status", "UNSAT")?;
            }
            other => {
                dict.set_item("status", format!("{}", other))?;
            }
        }
        Ok(dict.into_py(py))
    })
}

// ── LP solver ────────────────────────────────────────────────────────

/// Solve an LP problem from text.
#[pyfunction]
fn solve_lp_text(input: &str) -> PyResult<Py<PyAny>> {
    let parser = LpFormatParser::new();
    let problem = parser.parse(input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
    let result = crate::lp::solve_lp(&problem.system, &Config::default());

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        match result {
            LpResult::Optimal(model, obj) => {
                dict.set_item("status", "OPTIMAL")?;
                dict.set_item("model", model_to_py(&model.assignments))?;
                dict.set_item("objective", obj)?;
            }
            LpResult::Feasible(model) => {
                dict.set_item("status", "FEASIBLE")?;
                dict.set_item("model", model_to_py(&model.assignments))?;
            }
            LpResult::Infeasible => {
                dict.set_item("status", "INFEASIBLE")?;
            }
            LpResult::Unbounded => {
                dict.set_item("status", "UNBOUNDED")?;
            }
            other => {
                dict.set_item("status", format!("{}", other))?;
            }
        }
        Ok(dict.into_py(py))
    })
}

// ── MILP solver ──────────────────────────────────────────────────────

/// Solve a MILP problem from text.
#[pyfunction]
fn solve_milp_text(input: &str) -> PyResult<Py<PyAny>> {
    let parser = LpFormatParser::new();
    let problem = parser.parse(input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
    let result = crate::milp::solve_milp(&problem.system, &Config::default());

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        match result {
            LpResult::Optimal(model, obj) => {
                dict.set_item("status", "OPTIMAL")?;
                dict.set_item("model", model_to_py(&model.assignments))?;
                dict.set_item("objective", obj)?;
            }
            LpResult::Feasible(model) => {
                dict.set_item("status", "FEASIBLE")?;
                dict.set_item("model", model_to_py(&model.assignments))?;
            }
            LpResult::Infeasible => {
                dict.set_item("status", "INFEASIBLE")?;
            }
            LpResult::Unbounded => {
                dict.set_item("status", "UNBOUNDED")?;
            }
            other => {
                dict.set_item("status", format!("{}", other))?;
            }
        }
        Ok(dict.into_py(py))
    })
}

// ── CP solver ────────────────────────────────────────────────────────

/// Solve a CP problem from text.
#[pyfunction]
fn solve_cp_text(input: &str) -> PyResult<Py<PyAny>> {
    let parser = CpFormatParser::new();
    let problem = parser.parse(input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
    let result = crate::cp::solve_cp(&problem, &Config::default());

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        match result {
            SolverResult::Sat(model) => {
                dict.set_item("status", "SAT")?;
                dict.set_item("model", model_to_py(&model.assignments))?;
            }
            SolverResult::Unsat => {
                dict.set_item("status", "UNSAT")?;
            }
            other => {
                dict.set_item("status", format!("{}", other))?;
            }
        }
        Ok(dict.into_py(py))
    })
}
