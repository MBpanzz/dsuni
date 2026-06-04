//! Term normalisation and type checking.
//!
//! This pipeline transforms raw parsed terms into a canonical internal
//! representation before they reach a solver.

/// Placeholder — will be filled during Phase 1 development.
#[derive(Default)]
pub struct Normalizer;

impl Normalizer {
    pub fn new() -> Self {
        Self
    }

    /// Normalize a core `Term`.
    pub fn normalize(&self, _term: &crate::core::Term) -> crate::core::Term {
        unimplemented!("Normalizer not yet implemented")
    }
}
