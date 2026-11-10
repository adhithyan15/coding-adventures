//! # Reading the two inputs: a DIMACS formula and a DRAT proof
//!
//! The kernel reads its inputs with its own code. It shares nothing with
//! the solver's DIMACS reader, on purpose (VER11 §3.1): if both sides used
//! one parser, a parser bug could make the solver and the checker misread
//! a formula *in the same way*, and the checker would then happily certify
//! a proof about a formula nobody wrote.
//!
//! ## Literals
//!
//! Inside the kernel a literal is a `u32` code, `2·var + sign`, with `sign = 1`
//! for a negated literal and `var` 0-based. DIMACS `3` is code 4, `-3` is code 5.
//! [`neg`] flips the low bit, and [`var`] shifts it away.
//!
//! ## The formula: strict DIMACS CNF (VER09 §5)
//!
//! ```text
//!   c comment
//!   p cnf 3 2
//!   1 -3 0
//!   2 3 -1 0
//! ```
//!
//! One header before any clause; literals within the declared range; every
//! clause terminated by `0`; the clause count as declared; a line holding
//! only `%` ends the input.
//!
//! ## The proof: DRAT text
//!
//! ```text
//!   1 -2 0      add (x1 ∨ ¬x2)
//!   d 1 3 0     delete (x1 ∨ x3)
//!   0           add the empty clause
//! ```
//!
//! A step is an optional `d` followed by literals and a `0`. Lines starting
//! with `c` are comments. Steps may span lines, but a proof that ends in the
//! middle of a step is **truncated** and rejected. Proof literals may name
//! variables the formula does not have (RAT lemmas can introduce them), up
//! to the caller's variable limit.

use std::fmt;

/// A literal: `2·var + sign`.
pub type Lit = u32;

/// The negation of a literal.
pub fn neg(lit: Lit) -> Lit {
    lit ^ 1
}

/// The 0-based variable of a literal.
pub fn var(lit: Lit) -> usize {
    (lit >> 1) as usize
}

/// The strict integer grammar of VER11 3.4.2 rule L4: `-?(0|[1-9][0-9]*)` as
/// an `i64`, where `-0` is **not** an integer and no token has a leading zero
/// (`00` and `01` are rejected, so no reader can disagree about whether `00`
/// is a terminator).
///
/// Rust's own `i64::from_str` is more generous: it accepts a leading `+`, and
/// it reads `-0` as `0`. Taking either would let this reader disagree with
/// every other DIMACS reader about where a clause ends (`-0` would end one),
/// which is the exact parser differential the kernel's private parser exists
/// to prevent. So the token is vetted byte by byte first.
fn parse_int(token: &str) -> Option<i64> {
    let digits = token.strip_prefix('-').unwrap_or(token);
    if !is_canonical_digits(digits) {
        return None;
    }
    let value: i64 = token.parse().ok()?;
    if token.starts_with('-') && value == 0 {
        return None; // `-0`
    }
    Some(value)
}

/// `0` or a digit string with no leading zero.
fn is_canonical_digits(digits: &str) -> bool {
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
}

/// A non-negative count in a `p cnf` header: canonical ASCII digits only
/// (no `+`, no leading zeros).
fn parse_count(token: &str) -> Option<usize> {
    if !is_canonical_digits(token) {
        return None;
    }
    token.parse().ok()
}

/// DIMACS integer → literal code. The caller has checked `value != 0` and
/// `|value| <= limit`, and the limit keeps the code inside `u32`.
fn lit_from_dimacs(value: i64) -> Lit {
    let v = (value.unsigned_abs() - 1) as u32;
    (v << 1) | (value < 0) as u32
}

/// Literal code → DIMACS integer, for messages.
pub fn lit_to_dimacs(lit: Lit) -> i64 {
    let magnitude = var(lit) as i64 + 1;
    if lit & 1 == 1 {
        -magnitude
    } else {
        magnitude
    }
}

/// The largest variable limit the literal encoding can hold.
pub const MAX_VAR_LIMIT: usize = 1 << 30;

/// A parsed formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    /// Declared variable count.
    pub num_vars: usize,
    /// Clauses, as written.
    pub clauses: Vec<Vec<Lit>>,
}

/// Whether a proof step adds or deletes its clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Add a lemma; it must be RUP or RAT.
    Add,
    /// Delete a clause; it must be present.
    Delete,
}

/// One proof step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Add or delete.
    pub kind: StepKind,
    /// The clause, literals in the order written (RAT uses the first).
    pub lits: Vec<Lit>,
    /// 1-based line on which the step ends.
    pub line: usize,
}

/// Why an input could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The formula is not strict DIMACS CNF.
    Formula {
        /// 1-based line.
        line: usize,
        /// What is wrong.
        message: String,
    },
    /// A proof token is not an integer or a well-placed `d`.
    BadProofToken {
        /// 1-based line.
        line: usize,
        /// The token.
        token: String,
    },
    /// The proof ends inside a step.
    Truncated {
        /// The last line.
        line: usize,
    },
    /// A proof literal's variable is above the limit.
    ProofVarOverLimit {
        /// 1-based line.
        line: usize,
        /// The literal as written.
        literal: i64,
        /// The limit.
        limit: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Formula { line, message } => write!(f, "formula line {line}: {message}"),
            ParseError::BadProofToken { line, token } => {
                write!(f, "proof line {line}: unexpected token `{token}`")
            }
            ParseError::Truncated { line } => {
                write!(f, "proof line {line}: proof ends inside a step (truncated)")
            }
            ParseError::ProofVarOverLimit { line, literal, limit } => {
                write!(f, "proof line {line}: literal {literal} exceeds the variable limit {limit}")
            }
        }
    }
}

/// Lines that carry content: not blank, not a comment. Yields
/// `(1-based line number, trimmed text)` and stops at a `%` line when
/// `percent_ends` is set.
fn content_lines(text: &str, percent_ends: bool) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .take_while(move |(_, l)| !(percent_ends && *l == "%"))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('c'))
}

/// Read a strict DIMACS CNF formula with at most `max_vars` variables.
pub fn parse_formula(text: &str, max_vars: usize) -> Result<Formula, ParseError> {
    let fail = |line: usize, message: String| Err(ParseError::Formula { line, message });
    let mut header: Option<(usize, usize)> = None;
    let mut clauses = Vec::new();
    let mut current = Vec::new();
    let mut last = 1;
    for (line, text) in content_lines(text, true) {
        last = line;
        if text.starts_with('p') {
            if header.is_some() {
                return fail(line, "second `p` header".into());
            }
            let f: Vec<&str> = text.split_whitespace().collect();
            let counts = match f.as_slice() {
                ["p", "cnf", v, c] => parse_count(v).zip(parse_count(c)),
                _ => None,
            };
            match counts {
                Some((v, _)) if v > max_vars => {
                    return fail(line, format!("{v} variables exceeds the limit {max_vars}"));
                }
                Some(h) => header = Some(h),
                None => return fail(line, format!("malformed header `{text}`")),
            }
            continue;
        }
        let Some((num_vars, _)) = header else {
            return fail(line, "clause before the `p cnf` header".into());
        };
        for token in text.split_whitespace() {
            let Some(value) = parse_int(token) else {
                return fail(line, format!("`{token}` is not an integer"));
            };
            if value == 0 {
                clauses.push(std::mem::take(&mut current));
            } else if value.unsigned_abs() > num_vars as u64 {
                return fail(line, format!("literal {value} is outside {num_vars} variables"));
            } else {
                current.push(lit_from_dimacs(value));
            }
        }
    }
    let Some((num_vars, declared)) = header else {
        return fail(last, "no `p cnf` header".into());
    };
    if !current.is_empty() {
        return fail(last, "last clause has no terminating 0".into());
    }
    if clauses.len() != declared {
        return fail(last, format!("header declares {declared} clauses, found {}", clauses.len()));
    }
    Ok(Formula { num_vars, clauses })
}

/// Read a DRAT text proof whose literals stay within `max_vars` variables.
pub fn parse_proof(text: &str, max_vars: usize) -> Result<Vec<Step>, ParseError> {
    let mut steps = Vec::new();
    let mut lits = Vec::new();
    let mut deleting = false;
    let mut last = 1;
    for (line, text) in content_lines(text, false) {
        last = line;
        for token in text.split_whitespace() {
            if token == "d" && lits.is_empty() && !deleting {
                deleting = true;
                continue;
            }
            let value = parse_int(token)
                .ok_or_else(|| ParseError::BadProofToken { line, token: token.to_owned() })?;
            if value == 0 {
                let kind = if deleting { StepKind::Delete } else { StepKind::Add };
                steps.push(Step { kind, lits: std::mem::take(&mut lits), line });
                deleting = false;
            } else if value.unsigned_abs() > max_vars as u64 {
                return Err(ParseError::ProofVarOverLimit { line, literal: value, limit: max_vars });
            } else {
                lits.push(lit_from_dimacs(value));
            }
        }
    }
    if deleting || !lits.is_empty() {
        return Err(ParseError::Truncated { line: last });
    }
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_codes() {
        assert_eq!(lit_from_dimacs(1), 0);
        assert_eq!(lit_from_dimacs(-1), 1);
        assert_eq!(lit_from_dimacs(3), 4);
        assert_eq!(neg(4), 5);
        assert_eq!(var(5), 2);
        assert_eq!(lit_to_dimacs(5), -3);
        assert_eq!(lit_to_dimacs(4), 3);
    }

    /// Regression (VER11 D6): Rust's `i64::from_str` accepts `+1` and reads
    /// `-0` as `0`, which made `-0` a clause terminator.
    #[test]
    fn integer_tokens_are_strict() {
        for ok in ["0", "7", "-7", "9223372036854775807", "-9223372036854775808"] {
            assert!(parse_int(ok).is_some(), "{ok}");
        }
        assert_eq!(parse_int("-7"), Some(-7));
        for bad in ["", "-", "+", "+1", "+0", "-0", "-00", "00", "01", "007", "-01", "--1", "1-", "1.0", "1e3", "0x1", " 1", "1 ", "9223372036854775808", "-9223372036854775809", "\u{661}"] {
            assert_eq!(parse_int(bad), None, "{bad:?}");
        }
        assert_eq!(parse_count("12"), Some(12));
        assert_eq!(parse_count("0"), Some(0));
        for bad in ["", "012", "00", "+1", "-1", "1.0", "x", "99999999999999999999"] {
            assert_eq!(parse_count(bad), None, "{bad:?}");
        }
        // end to end: `-0` must not split a clause or end one.
        assert!(parse_formula("p cnf 1 1\n-0 1 0\n", 10).is_err());
        assert!(parse_proof("1 -0\n", 10).is_err());
    }

    #[test]
    fn formula_ok() {
        let f = parse_formula("c x\np cnf 3 2\n1 -3 0 2\n3 -1 0\n%\njunk\n", 10).unwrap();
        assert_eq!(f.num_vars, 3);
        assert_eq!(f.clauses, vec![vec![0, 5], vec![2, 4, 1]]);
    }

    #[test]
    fn formula_errors() {
        let msg = |t: &str| match parse_formula(t, 10).unwrap_err() {
            ParseError::Formula { message, .. } => message,
            other => panic!("{other:?}"),
        };
        assert!(msg("p cnf 1 0\np cnf 1 0\n").contains("second"));
        assert!(msg("p cnf 11 0\n").contains("limit"));
        assert!(msg("p cnf x 0\n").contains("malformed"));
        assert!(msg("p dnf 1 0\n").contains("malformed"));
        assert!(msg("1 0\n").contains("before"));
        assert!(msg("p cnf 1 1\nq 0\n").contains("not an integer"));
        assert!(msg("p cnf 1 1\n2 0\n").contains("outside"));
        assert!(msg("p cnf 1 1\n-9223372036854775808 0\n").contains("outside"));
        assert!(msg("").contains("no `p cnf`"));
        assert!(msg("p cnf 1 1\n1\n").contains("terminating"));
        assert!(msg("p cnf 1 2\n1 0\n").contains("declares 2"));
        let shown = parse_formula("p cnf 1 1\n\n2 0\n", 10).unwrap_err().to_string();
        assert_eq!(shown, "formula line 3: literal 2 is outside 1 variables");
    }

    #[test]
    fn proof_ok() {
        let steps = parse_proof("c hi\n1 -2 0 d 1\n 3 0\n0\n", 5).unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], Step { kind: StepKind::Add, lits: vec![0, 3], line: 2 });
        assert_eq!(steps[1], Step { kind: StepKind::Delete, lits: vec![0, 4], line: 3 });
        assert_eq!(steps[2], Step { kind: StepKind::Add, lits: vec![], line: 4 });
        assert_eq!(parse_proof("", 5).unwrap(), vec![]);
    }

    #[test]
    fn proof_errors() {
        assert_eq!(
            parse_proof("1 d 0\n", 5).unwrap_err(),
            ParseError::BadProofToken { line: 1, token: "d".into() }
        );
        assert_eq!(
            parse_proof("d d 1 0\n", 5).unwrap_err(),
            ParseError::BadProofToken { line: 1, token: "d".into() }
        );
        assert_eq!(parse_proof("1 2 0\n3", 5).unwrap_err(), ParseError::Truncated { line: 2 });
        assert_eq!(parse_proof("0\nd\n", 5).unwrap_err(), ParseError::Truncated { line: 2 });
        assert_eq!(
            parse_proof("6 0\n", 5).unwrap_err(),
            ParseError::ProofVarOverLimit { line: 1, literal: 6, limit: 5 }
        );
        for e in [
            ParseError::BadProofToken { line: 1, token: "x".into() },
            ParseError::Truncated { line: 2 },
            ParseError::ProofVarOverLimit { line: 3, literal: -9, limit: 5 },
        ] {
            assert!(e.to_string().starts_with("proof line"), "{e}");
        }
    }
}
