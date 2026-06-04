//! MiniZinc-subset parser (CP).

/// Placeholder.
pub struct MznParser;

impl MznParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, _input: &str) -> super::ParseResult<String> {
        Err("MiniZinc parser not yet implemented".into())
    }
}
