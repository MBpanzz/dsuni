//! MPS format parser (MILP).

/// Placeholder.
pub struct MpsParser;

impl MpsParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, _input: &str) -> super::ParseResult<String> {
        Err("MPS parser not yet implemented".into())
    }
}
