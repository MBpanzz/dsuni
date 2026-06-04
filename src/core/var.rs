/// A unique variable identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Var(pub usize);

impl Var {
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x{}", self.0)
    }
}
