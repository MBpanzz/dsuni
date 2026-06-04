/// Global configuration passed to every solver.
#[derive(Debug, Clone)]
pub struct Config {
    /// Time limit in milliseconds (None = no limit).
    pub timeout_ms: Option<u64>,
    /// Memory limit in MB (None = no limit).
    pub max_memory_mb: Option<usize>,
    /// Verbosity level (0 = silent, 1 = normal, 2+ = debug).
    pub verbosity: u8,
    /// Optional random seed for reproducible runs.
    pub random_seed: Option<u64>,
    /// Whether to verify models against constraints before returning.
    pub check_model: bool,
    /// Whether to produce a proof trace.
    pub produce_proof: bool,
    /// Whether to support unsat-core extraction.
    pub produce_unsat_core: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            max_memory_mb: None,
            verbosity: 1,
            random_seed: None,
            check_model: true,
            produce_proof: false,
            produce_unsat_core: false,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style: set timeout.
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    /// Builder-style: set verbosity.
    pub fn with_verbosity(mut self, level: u8) -> Self {
        self.verbosity = level;
        self
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.verbosity, 1);
        assert!(cfg.timeout_ms.is_none());
        assert!(cfg.check_model);
        assert!(!cfg.produce_proof);
    }

    #[test]
    fn test_builder_methods() {
        let cfg = Config::new()
            .with_timeout(5000)
            .with_verbosity(2);
        assert_eq!(cfg.timeout_ms, Some(5000));
        assert_eq!(cfg.verbosity, 2);
    }
}
