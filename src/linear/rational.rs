/// A rational number represented as a reduced fraction `n / d` (d > 0).
///
/// Uses `i64` for both numerator and denominator. Arithmetic is exact;
/// division by zero panics at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i64,
    den: i64, // always > 0
}

impl Rational {
    /// The rational number zero.
    pub const ZERO: Rational = Rational { num: 0, den: 1 };
    /// The rational number one.
    pub const ONE: Rational = Rational { num: 1, den: 1 };
    /// Negative one.
    pub const NEG_ONE: Rational = Rational { num: -1, den: 1 };

    /// Create a new rational number `n / d`.
    ///
    /// # Panics
    /// Panics if `d == 0`.
    pub fn new(n: i64, d: i64) -> Self {
        assert!(d != 0, "Rational denominator cannot be zero");
        let mut r = Rational { num: n, den: d };
        r.normalize();
        r
    }

    /// Numerator.
    #[inline]
    pub fn num(&self) -> i64 {
        self.num
    }

    /// Denominator (always > 0).
    #[inline]
    pub fn den(&self) -> i64 {
        self.den
    }

    /// Reduce the fraction to lowest terms and ensure denominator > 0.
    fn normalize(&mut self) {
        if self.den < 0 {
            self.num = -self.num;
            self.den = -self.den;
        }
        let g = gcd(self.num.unsigned_abs(), self.den as u64) as i64;
        if g > 1 {
            self.num /= g;
            self.den /= g;
        }
    }

    /// Is this rational zero?
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    /// Is this rational one?
    #[inline]
    pub fn is_one(&self) -> bool {
        self.num == self.den
    }

    /// Is this rational negative?
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.num < 0
    }

    /// Is this rational positive?
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.num > 0
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        Rational::new(self.num.unsigned_abs() as i64, self.den)
    }

    /// Negation.
    pub fn neg(&self) -> Self {
        Rational::new(-self.num, self.den)
    }

    /// Reciprocal.
    ///
    /// # Panics
    /// Panics if `self == 0`.
    pub fn recip(&self) -> Self {
        assert!(!self.is_zero(), "cannot invert zero");
        Rational::new(self.den, self.num)
    }

    /// Convert to `f64` (may lose precision).
    pub fn to_f64(&self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

// ── arithmetic operators ──────────────────────────────────────────────

impl std::ops::Add for Rational {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Rational::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }
}

impl std::ops::Sub for Rational {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Rational::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        )
    }
}

impl std::ops::Mul for Rational {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Rational::new(self.num * other.num, self.den * other.den)
    }
}

impl std::ops::Div for Rational {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        assert!(!other.is_zero(), "division by zero");
        Rational::new(self.num * other.den, self.den * other.num)
    }
}

impl std::ops::Neg for Rational {
    type Output = Self;
    fn neg(self) -> Self {
        Rational::new(-self.num, self.den)
    }
}

// ── compound assignment ──────────────────────────────────────────────

impl std::ops::AddAssign for Rational {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl std::ops::SubAssign for Rational {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl std::ops::MulAssign for Rational {
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}

impl std::ops::DivAssign for Rational {
    fn div_assign(&mut self, other: Self) {
        *self = *self / other;
    }
}

// ── comparison ───────────────────────────────────────────────────────

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare cross-multiplied: a.num * b.den vs b.num * a.den
        let lhs = self.num * other.den;
        let rhs = other.num * self.den;
        lhs.cmp(&rhs)
    }
}

// ── conversions ───────────────────────────────────────────────────────

impl From<i64> for Rational {
    fn from(n: i64) -> Self {
        Rational::new(n, 1)
    }
}

impl From<i32> for Rational {
    fn from(n: i32) -> Self {
        Rational::new(n as i64, 1)
    }
}

impl From<u64> for Rational {
    fn from(n: u64) -> Self {
        Rational::new(n as i64, 1)
    }
}

impl std::fmt::Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────

/// Greatest common divisor (Euclidean algorithm).
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_construction() {
        let r = Rational::new(3, 4);
        assert_eq!(r.num(), 3);
        assert_eq!(r.den(), 4);
    }

    #[test]
    fn test_normalization() {
        let r = Rational::new(6, 8);
        assert_eq!(r.num(), 3);
        assert_eq!(r.den(), 4);
    }

    #[test]
    fn test_negative_denominator() {
        let r = Rational::new(3, -4);
        assert_eq!(r.num(), -3);
        assert_eq!(r.den(), 4);
    }

    #[test]
    fn test_zero() {
        assert!(Rational::ZERO.is_zero());
        assert_eq!(Rational::ZERO.num(), 0);
        assert_eq!(Rational::ZERO.den(), 1);
    }

    #[test]
    fn test_one() {
        assert!(Rational::ONE.is_one());
        assert_eq!(Rational::new(5, 5), Rational::ONE);
    }

    #[test]
    fn test_addition() {
        let a = Rational::new(1, 2);
        let b = Rational::new(1, 3);
        assert_eq!(a + b, Rational::new(5, 6));
    }

    #[test]
    fn test_subtraction() {
        let a = Rational::new(3, 4);
        let b = Rational::new(1, 4);
        assert_eq!(a - b, Rational::new(1, 2));
    }

    #[test]
    fn test_multiplication() {
        let a = Rational::new(2, 3);
        let b = Rational::new(3, 4);
        assert_eq!(a * b, Rational::new(1, 2));
    }

    #[test]
    fn test_division() {
        let a = Rational::new(2, 3);
        let b = Rational::new(3, 4);
        assert_eq!(a / b, Rational::new(8, 9));
    }

    #[test]
    fn test_negation() {
        let r = Rational::new(3, 5);
        assert_eq!(-r, Rational::new(-3, 5));
    }

    #[test]
    fn test_abs() {
        assert_eq!(Rational::new(-3, 4).abs(), Rational::new(3, 4));
    }

    #[test]
    fn test_comparison() {
        assert!(Rational::new(1, 3) < Rational::new(1, 2));
        assert!(Rational::new(5, 2) > Rational::new(2, 1));
        assert_eq!(Rational::new(2, 4), Rational::new(3, 6));
    }

    #[test]
    fn test_from_integer() {
        assert_eq!(Rational::from(5), Rational::new(5, 1));
        assert_eq!(Rational::from(-3i32), Rational::new(-3, 1));
    }

    #[test]
    fn test_to_f64() {
        let r = Rational::new(1, 2);
        assert!((r.to_f64() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_compound_assign() {
        let mut r = Rational::new(1, 2);
        r += Rational::new(1, 3);
        assert_eq!(r, Rational::new(5, 6));
        r *= Rational::new(2, 1);
        assert_eq!(r, Rational::new(5, 3));
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(7, 3), 1);
        assert_eq!(gcd(0, 5), 5);
    }
}
