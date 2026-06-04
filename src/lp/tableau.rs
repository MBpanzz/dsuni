use crate::linear::Rational;

/// A dense simplex tableau for exact rational LP solving.
///
/// Layout:
/// - Row 0: objective row (reduced costs, last element = current objective value)
/// - Rows 1..m: constraint rows (coefficients, last element = RHS)
/// - Columns 0..n-1: structural (decision) variables
/// - Columns n..n+m-1: slack / surplus / artificial variables
/// - Last column: RHS / objective value
///
/// `basic[row]` = index of the basic variable for constraint row `row`
/// (1-indexed, 0 = not a basic variable).
/// `basic[0]` is unused (objective row has no basic variable).
#[derive(Debug, Clone)]
pub struct Tableau {
    /// Number of structural (decision) variables.
    pub num_struct: usize,
    /// Number of constraints = rows - 1.
    pub num_constraints: usize,
    /// Total columns = num_struct + num_constraints + 1 (RHS).
    pub num_cols: usize,
    /// The tableau data: `data[row][col]` for row 0..num_constraints, col 0..num_cols.
    data: Vec<Vec<Rational>>,
    /// `basic[row]` = column index of the basic variable in constraint row `row` (1-indexed).
    /// Row 0 (objective) has no basic variable (stays None).
    pub basic: Vec<Option<usize>>,
}

impl Tableau {
    /// Create a new empty tableau for a standard-form LP.
    ///
    /// `num_struct` = number of structural variables (originals + slack/surplus/artificial).
    /// `num_constraints` = number of equality constraints.
    ///
    /// Objective row is all zeros. Constraint rows are all zeros — caller must
    /// fill in coefficients and set basic variables via [`set_coeff`] / [`set_rhs`] / [`set_basic`].
    pub fn new(num_struct: usize, num_constraints: usize) -> Self {
        let num_cols = num_struct + num_constraints + 1; // +1 for RHS
        let total_rows = num_constraints + 1; // +1 for objective row

        let mut data = Vec::with_capacity(total_rows);
        data.push(vec![Rational::ZERO; num_cols]);

        for _ in 0..num_constraints {
            data.push(vec![Rational::ZERO; num_cols]);
        }

        // No pre-set basic variables; caller must call set_basic for each row.
        let basic = vec![None; total_rows];

        Tableau {
            num_struct,
            num_constraints,
            num_cols,
            data,
            basic,
        }
    }

    /// Set which variable is basic in a constraint row (1-indexed).
    #[inline]
    pub fn set_basic(&mut self, row: usize, col: usize) {
        self.basic[row] = Some(col);
    }

    /// Number of rows (objective + constraints).
    #[inline]
    pub fn num_rows(&self) -> usize {
        self.num_constraints + 1
    }

    /// Get a reference to a cell.
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> &Rational {
        &self.data[row][col]
    }

    /// Get a mutable reference to a cell.
    #[inline]
    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut Rational {
        &mut self.data[row][col]
    }

    /// Get a reference to a full row.
    #[inline]
    pub fn row(&self, row: usize) -> &[Rational] {
        &self.data[row]
    }

    /// Get a mutable reference to a full row.
    #[inline]
    pub fn row_mut(&mut self, row: usize) -> &mut [Rational] {
        &mut self.data[row]
    }

    /// Get the RHS value for a constraint row (or objective row).
    #[inline]
    pub fn rhs(&self, row: usize) -> &Rational {
        &self.data[row][self.num_cols - 1]
    }

    /// RHS value for constraint `i` (1-indexed, row i+1).
    #[inline]
    pub fn constraint_rhs(&self, constraint: usize) -> &Rational {
        &self.data[constraint + 1][self.num_cols - 1]
    }

    /// Set the coefficient in the constraint matrix at (row, col).
    /// Row is 1-indexed constraint, col is the column.
    pub fn set_coeff(&mut self, constraint: usize, col: usize, value: Rational) {
        self.data[constraint + 1][col] = value;
    }

    /// Set the RHS for a constraint.
    pub fn set_rhs(&mut self, constraint: usize, value: Rational) {
        self.data[constraint + 1][self.num_cols - 1] = value;
    }

    /// Set the objective coefficient for a structural variable.
    pub fn set_obj_coeff(&mut self, col: usize, value: Rational) {
        self.data[0][col] = value;
    }

    /// Perform a pivot: make `entering_col` basic in `leaving_row`.
    ///
    /// `leaving_row` is 1-indexed (constraint row, not objective row).
    pub fn pivot(&mut self, leaving_row: usize, entering_col: usize) {
        let pivot_val = self.data[leaving_row][entering_col].clone();
        assert!(!pivot_val.is_zero(), "pivot value is zero");

        // Divide the pivot row by the pivot value.
        let pivot_row = leaving_row;
        for col in 0..self.num_cols {
            self.data[pivot_row][col] = self.data[pivot_row][col].clone() / pivot_val;
        }
        // The pivot element should now be exactly 1.
        debug_assert_eq!(self.data[pivot_row][entering_col], Rational::ONE);

        // Eliminate entering_col from all other rows.
        for row in 0..self.num_rows() {
            if row == pivot_row {
                continue;
            }
            let factor = self.data[row][entering_col].clone();
            if factor.is_zero() {
                continue;
            }
            for col in 0..self.num_cols {
                self.data[row][col] = self.data[row][col].clone() - factor.clone() * self.data[pivot_row][col].clone();
            }
        }

        // Update basic variable tracking.
        self.basic[pivot_row] = Some(entering_col);
    }

    /// Find the entering variable.
    ///
    /// In the `z - c^T x = 0` formulation, the coefficient in row 0 is `-c_j`.
    /// For minimization, the reduced cost is `c_j` and we want to enter
    /// variables with POSITIVE coefficients (most positive first).
    ///
    /// Returns `None` if all coefficients are ≤ 0 (optimal for minimization).
    pub fn find_entering(&self) -> Option<usize> {
        let mut best_col = None;
        let mut best_val = Rational::ZERO;
        for col in 0..self.num_cols - 1 {
            let val = &self.data[0][col];
            if val.is_positive() && (best_col.is_none() || *val > best_val) {
                best_col = Some(col);
                best_val = val.clone();
            }
        }
        best_col
    }

    /// Find the leaving variable using the minimum ratio test.
    ///
    /// `entering_col` is the column entering the basis.
    /// Returns `(row, basic_var_col)` or `None` if unbounded.
    /// Entries with coefficient ≤ 0 are skipped (unbounded direction).
    pub fn find_leaving(&self, entering_col: usize) -> Option<(usize, usize)> {
        let mut best_row = None;
        let mut best_ratio: Option<Rational> = None;

        for row in 1..self.num_rows() {
            let a = &self.data[row][entering_col];
            if a.is_positive() {
                let rhs = &self.data[row][self.num_cols - 1];
                let ratio = rhs.clone() / a.clone();
                if best_ratio.is_none() || ratio < best_ratio.clone().unwrap() {
                    best_ratio = Some(ratio);
                    best_row = Some(row);
                }
            }
        }

        best_row.and_then(|row| self.basic[row].map(|col| (row, col)))
    }

    /// Current objective value (from row 0, last column).
    ///
    /// The tableau stores `z + Σ r_j x_j = RHS`. At the optimal vertex
    /// all non-basic variables are 0, so `z = RHS`.
    pub fn objective_value(&self) -> Rational {
        self.data[0][self.num_cols - 1].clone()
    }

    /// Get the value of a variable from the tableau.
    /// For structural variables: if it's basic, the RHS of its row; otherwise 0.
    pub fn variable_value(&self, col: usize) -> Rational {
        for row in 1..self.num_rows() {
            if self.basic[row] == Some(col) {
                return self.data[row][self.num_cols - 1].clone();
            }
        }
        Rational::ZERO // non-basic variable is 0
    }

    /// Print tableau (for debugging).
    #[allow(dead_code)]
    pub fn debug_print(&self) {
        print!("  |");
        for col in 0..self.num_cols - 1 {
            print!(" {:>6}", col);
        }
        println!(" | {:>6}", "RHS");
        println!("--+{:-^6}+------", "");
        for row in 0..self.num_rows() {
            if row == 0 {
                print!("z |");
            } else {
                print!("{} |", self.basic[row].map(|c| format!("x{}", c)).unwrap_or_default());
            }
            for col in 0..self.num_cols - 1 {
                let v = &self.data[row][col];
                if v.is_zero() {
                    print!(" {:>6}", "·");
                } else {
                    print!(" {:>6}", v);
                }
            }
            println!(" | {:>6}", self.data[row][self.num_cols - 1]);
        }
        println!();
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tableau() {
        let t = Tableau::new(2, 3);
        assert_eq!(t.num_struct, 2);
        assert_eq!(t.num_constraints, 3);
        assert_eq!(t.num_cols, 2 + 3 + 1);
        assert_eq!(t.num_rows(), 4);
    }

    #[test]
    fn test_set_basic() {
        let mut t = Tableau::new(3, 2);
        t.set_basic(1, 3);
        t.set_basic(2, 4);
        assert_eq!(t.basic[1], Some(3));
        assert_eq!(t.basic[2], Some(4));
    }

    #[test]
    fn test_pivot_identity() {
        let mut t = Tableau::new(2, 1); // 2 struct vars (x0 + slack)
        // Row 1 (constraint 0): x0 + slack = 5
        t.set_coeff(0, 0, Rational::new(1, 1));  // x0 coefficient
        t.set_coeff(0, 1, Rational::new(1, 1));  // slack coefficient
        t.set_rhs(0, Rational::new(5, 1));
        t.set_basic(1, 1); // slack (col 1) is basic

        // Objective row: z + x0 = 0  (for min -x0, so -c_0 = -(-1) = 1)
        t.set_obj_coeff(0, Rational::new(1, 1));

        // Verify initial layout
        assert_eq!(t.get(1, 0), &Rational::new(1, 1));   // coeff of x0
        assert_eq!(t.get(1, 1), &Rational::new(1, 1));   // coeff of slack
        assert_eq!(t.rhs(1), &Rational::new(5, 1));

        // Pivot on col 0 (x0 enters), row 1 (slack leaves)
        t.pivot(1, 0);
        // After pivot:
        // obj:  [0, 1 | 5]  (reduced costs, RHS = -z = 5? Let me check)
        // row1: [1, 1 | 5]
        assert_eq!(t.get(0, 0), &Rational::ZERO);   // x0 eliminated from obj
        assert_eq!(t.basic[1], Some(0));              // x0 is now basic
    }

    #[test]
    fn test_find_entering() {
        let mut t = Tableau::new(2, 1);
        t.set_obj_coeff(0, Rational::new(3, 1));  // positive → candidate
        t.set_obj_coeff(1, Rational::new(1, 1));  // positive, but smaller

        let entering = t.find_entering();
        assert_eq!(entering, Some(0)); // 3 > 1
    }

    #[test]
    fn test_find_entering_no_candidate() {
        let t = Tableau::new(2, 1);
        // All coefficients negative or zero → optimal
        assert_eq!(t.find_entering(), None);
    }

    #[test]
    fn test_find_entering_negative_optimal() {
        let mut t = Tableau::new(2, 1);
        t.set_obj_coeff(0, Rational::new(-3, 1)); // negative → not entering
        assert_eq!(t.find_entering(), None);
    }

    #[test]
    fn test_find_leaving() {
        let mut t = Tableau::new(2, 2); // 2 struct vars, 2 constraints
        // Slack columns are at col 2 and col 3 (num_struct + constraint_index)
        t.set_basic(1, 2); // row 1: basic slack at col 2
        t.set_basic(2, 3); // row 2: basic slack at col 3
        t.set_coeff(0, 0, Rational::new(2, 1));
        t.set_coeff(0, 1, Rational::new(1, 1));
        t.set_rhs(0, Rational::new(10, 1));
        t.set_coeff(1, 0, Rational::new(1, 1));
        t.set_coeff(1, 1, Rational::new(3, 1));
        t.set_rhs(1, Rational::new(12, 1));

        // For entering col 0:
        // Row 1: ratio = 10/2 = 5
        // Row 2: ratio = 12/1 = 12
        // Minimum ratio → row 1, basic slack at col 2
        let leaving = t.find_leaving(0);
        assert_eq!(leaving, Some((1, 2)));
    }

    #[test]
    fn test_objective_value() {
        let t = Tableau::new(2, 1);
        assert_eq!(t.objective_value(), Rational::ZERO);
    }
}
