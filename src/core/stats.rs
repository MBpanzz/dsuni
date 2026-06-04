/// Solver statistics collected during a run.
#[derive(Debug, Clone, Default)]
pub struct SolverStats {
    /// Wall-clock elapsed time in milliseconds.
    pub elapsed_ms: u64,
    /// Number of decisions made.
    pub decisions: u64,
    /// Number of propagations.
    pub propagations: u64,
    /// Number of conflicts.
    pub conflicts: u64,
    /// Number of restarts.
    pub restarts: u64,
    /// Number of search nodes explored (MILP/CP).
    pub nodes: u64,
    /// Number of simplex iterations (LP).
    pub simplex_iterations: u64,
    /// Peak memory usage in MB, if available.
    pub memory_mb: Option<usize>,
}

impl SolverStats {
    pub fn new() -> Self {
        Self::default()
    }
}

impl std::fmt::Display for SolverStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  elapsed: {} ms", self.elapsed_ms)?;
        writeln!(f, "  decisions: {}", self.decisions)?;
        writeln!(f, "  propagations: {}", self.propagations)?;
        writeln!(f, "  conflicts: {}", self.conflicts)?;
        writeln!(f, "  restarts: {}", self.restarts)?;
        writeln!(f, "  nodes: {}", self.nodes)?;
        writeln!(f, "  simplex iterations: {}", self.simplex_iterations)?;
        if let Some(mem) = self.memory_mb {
            writeln!(f, "  memory: {} MB", mem)?;
        }
        Ok(())
    }
}
