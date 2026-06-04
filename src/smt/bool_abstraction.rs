use crate::sat::Lit;
use crate::smt::TheoryAtom;
use crate::smt::TheoryLit;

/// Maps theory atoms to SAT variables.
///
/// Each theory atom becomes a SAT variable:
/// - Atom a_i → SAT variable i
/// - TheoryLit { atom_id: i, sign: true } → Lit::positive(i)
/// - TheoryLit { atom_id: i, sign: false } → Lit::negative(i)
#[derive(Debug, Clone)]
pub struct BoolAbstraction {
    /// The list of theory atoms.
    atoms: Vec<Box<dyn TheoryAtom>>,
}

impl BoolAbstraction {
    pub fn new() -> Self {
        BoolAbstraction { atoms: Vec::new() }
    }

    /// Register a theory atom and return its ID.
    pub fn register_atom(&mut self, atom: Box<dyn TheoryAtom>) -> usize {
        let id = self.atoms.len();
        self.atoms.push(atom);
        id
    }

    /// Get the number of atoms registered.
    pub fn num_atoms(&self) -> usize {
        self.atoms.len()
    }

    /// Convert a theory literal to a SAT literal.
    pub fn to_sat_lit(&self, tlit: TheoryLit) -> Lit {
        if tlit.sign {
            Lit::positive(tlit.atom_id)
        } else {
            Lit::negative(tlit.atom_id)
        }
    }

    /// Convert a SAT literal back to a theory literal.
    pub fn to_theory_lit(&self, sat_lit: Lit) -> TheoryLit {
        TheoryLit {
            atom_id: sat_lit.var,
            sign: sat_lit.sign,
        }
    }

    /// Get the theory atom by ID.
    pub fn get_atom(&self, id: usize) -> &dyn TheoryAtom {
        self.atoms[id].as_ref()
    }

    /// Iterate over all atoms.
    pub fn atoms(&self) -> impl Iterator<Item = &dyn TheoryAtom> {
        self.atoms.iter().map(|b| b.as_ref())
    }
}

impl Default for BoolAbstraction {
    fn default() -> Self {
        Self::new()
    }
}
