//! # Variables, literals and three-valued truth
//!
//! A SAT solver spends almost all of its time asking one question, millions
//! of times a second: *what is the value of this literal right now?* So the
//! representation of a literal is chosen to make that question one array
//! index and one XOR.
//!
//! ## Variables
//!
//! A [`Var`] is just a dense index `0, 1, 2, …`. Dense, so that per-variable
//! facts (value, decision level, reason, activity) live in plain `Vec`s
//! indexed by the variable, with no hashing.
//!
//! ## Literals: one integer, sign in the low bit
//!
//! A [`Lit`] is a variable together with a polarity. It is packed into one
//! `u32` as `2·var + sign`, where `sign = 1` means *negated*:
//!
//! ```text
//!   variable   positive literal   negative literal
//!   x0         0                  1
//!   x1         2                  3
//!   x2         4                  5
//!   xv         2v                 2v + 1
//! ```
//!
//! That packing buys three things at once:
//!
//! | Operation | Cost |
//! |---|---|
//! | negate a literal | `code ^ 1` |
//! | variable of a literal | `code >> 1` |
//! | index an array by literal (watch lists) | `code as usize` |
//!
//! This is the encoding MiniSat made standard, and every CDCL solver since
//! has used it or something isomorphic to it.
//!
//! ## DIMACS numbering
//!
//! The textual DIMACS format numbers variables from **1** and writes a
//! negative literal with a minus sign: `-3` is "not x3". Variable 0 does not
//! exist there, because `0` terminates a clause. Conversion between the two
//! numberings happens only at the boundary ([`Lit::from_dimacs`],
//! [`Lit::to_dimacs`]); inside the solver everything is 0-based.
//!
//! ```text
//!   DIMACS  1  ↔  Var(0) positive  ↔  code 0
//!   DIMACS -1  ↔  Var(0) negative  ↔  code 1
//!   DIMACS  2  ↔  Var(1) positive  ↔  code 2
//! ```

use std::fmt;
use std::ops::Not;

/// A propositional variable, identified by a dense 0-based index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var(u32);

impl Var {
    /// The variable with 0-based index `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` does not fit the literal encoding (`index` must be
    /// below `2^31`, because a literal stores `2·index + 1` in a `u32`).
    pub fn from_index(index: usize) -> Var {
        assert!(index < (1 << 31), "variable index {index} exceeds 2^31 - 1");
        Var(index as u32)
    }

    /// The 0-based index of this variable.
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// The positive literal of this variable: `x`.
    pub fn positive(self) -> Lit {
        Lit(self.0 << 1)
    }

    /// The negative literal of this variable: `¬x`.
    pub fn negative(self) -> Lit {
        Lit((self.0 << 1) | 1)
    }

    /// The literal of this variable with the given polarity: `x` if
    /// `negated` is false, `¬x` if it is true.
    pub fn lit(self, negated: bool) -> Lit {
        Lit((self.0 << 1) | negated as u32)
    }
}

/// A literal: a variable or its negation, packed as `2·var + sign`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lit(u32);

impl Lit {
    /// The variable this literal mentions.
    pub fn var(self) -> Var {
        Var(self.0 >> 1)
    }

    /// Whether this is the negative literal `¬x`.
    pub fn is_negated(self) -> bool {
        self.0 & 1 == 1
    }

    /// The packed code `2·var + sign`, usable as an array index.
    pub fn code(self) -> usize {
        self.0 as usize
    }

    /// Rebuild a literal from its packed code.
    pub fn from_code(code: usize) -> Lit {
        assert!(code <= u32::MAX as usize, "literal code {code} exceeds u32");
        Lit(code as u32)
    }

    /// Convert a DIMACS literal (1-based, minus sign for negation).
    ///
    /// Returns `None` for `0`, which in DIMACS is the clause terminator and
    /// not a literal, and for values whose variable does not fit the
    /// encoding.
    pub fn from_dimacs(value: i64) -> Option<Lit> {
        if value == 0 {
            return None;
        }
        let magnitude = value.unsigned_abs();
        if magnitude > (1 << 31) {
            return None;
        }
        let var = Var((magnitude - 1) as u32);
        Some(var.lit(value < 0))
    }

    /// Convert to DIMACS numbering: variable index + 1, negative if negated.
    pub fn to_dimacs(self) -> i64 {
        let magnitude = self.var().index() as i64 + 1;
        if self.is_negated() {
            -magnitude
        } else {
            magnitude
        }
    }
}

impl Not for Lit {
    type Output = Lit;

    /// Negation flips the low bit: `x ↔ ¬x`.
    fn not(self) -> Lit {
        Lit(self.0 ^ 1)
    }
}

impl fmt::Display for Lit {
    /// Literals print in DIMACS numbering, the notation every SAT paper and
    /// tool uses: `3`, `-3`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_dimacs())
    }
}

/// Three-valued truth: a variable is true, false, or not yet assigned.
///
/// During search most variables are `Undef`. The solver's whole job is to
/// move them to `True` or `False` without making any clause all-`False`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LBool {
    /// Assigned true.
    True,
    /// Assigned false.
    False,
    /// Not assigned.
    Undef,
}

impl LBool {
    /// `True` for `true`, `False` for `false`.
    pub fn from_bool(b: bool) -> LBool {
        if b {
            LBool::True
        } else {
            LBool::False
        }
    }

    /// Flip `True ↔ False`; `Undef` stays `Undef`. This is how a variable's
    /// value becomes a negative literal's value.
    pub fn negate_if(self, flip: bool) -> LBool {
        match (self, flip) {
            (LBool::True, true) => LBool::False,
            (LBool::False, true) => LBool::True,
            (v, _) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_matches_the_table() {
        let x0 = Var::from_index(0);
        let x2 = Var::from_index(2);
        assert_eq!(x0.positive().code(), 0);
        assert_eq!(x0.negative().code(), 1);
        assert_eq!(x2.positive().code(), 4);
        assert_eq!(x2.negative().code(), 5);
        assert_eq!(x2.lit(false), x2.positive());
        assert_eq!(x2.lit(true), x2.negative());
        assert_eq!(Lit::from_code(5), x2.negative());
    }

    #[test]
    fn negation_is_an_involution() {
        let l = Var::from_index(7).positive();
        assert_eq!(!l, Var::from_index(7).negative());
        assert_eq!(!!l, l);
        assert!(!l.is_negated());
        assert!((!l).is_negated());
        assert_eq!((!l).var(), l.var());
    }

    #[test]
    fn dimacs_round_trip() {
        for d in [-5i64, -1, 1, 2, 17] {
            let l = Lit::from_dimacs(d).unwrap();
            assert_eq!(l.to_dimacs(), d);
            assert_eq!(format!("{l}"), d.to_string());
        }
        assert_eq!(Lit::from_dimacs(1), Some(Var::from_index(0).positive()));
        assert_eq!(Lit::from_dimacs(-1), Some(Var::from_index(0).negative()));
        assert_eq!(Lit::from_dimacs(0), None);
        assert_eq!(Lit::from_dimacs(i64::MIN), None);
        assert_eq!(Lit::from_dimacs((1 << 31) + 1), None);
        assert!(Lit::from_dimacs(1 << 31).is_some());
    }

    #[test]
    #[should_panic(expected = "exceeds 2^31")]
    fn oversized_variable_panics() {
        let _ = Var::from_index(1 << 31);
    }

    #[test]
    fn lbool_negation() {
        assert_eq!(LBool::from_bool(true), LBool::True);
        assert_eq!(LBool::from_bool(false), LBool::False);
        assert_eq!(LBool::True.negate_if(true), LBool::False);
        assert_eq!(LBool::False.negate_if(true), LBool::True);
        assert_eq!(LBool::Undef.negate_if(true), LBool::Undef);
        assert_eq!(LBool::True.negate_if(false), LBool::True);
    }
}
