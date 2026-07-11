//! The PICTURE type model — the foundation of COBOL's data quirks.
//!
//! A picture describes a fixed-size field. Its **size** is the number of
//! storage-bearing character positions; a numeric field's implied decimal point
//! (`V`) and default operational sign (`S`) occupy none.
//!
//! v0.1 models three pure categories — unsigned numeric-display (`9` with an
//! optional `V`), alphanumeric (`X`), and alphabetic (`A`) — each with `(n)`
//! repetition. Everything else (`S`, `P`, editing symbols, mixed classes)
//! returns [`RuntimeError::UnsupportedPicture`]; those are the next PRs.

use crate::error::RuntimeError;

/// Maximum picture size (character positions). COBOL-85 caps a picture at 65535
/// positions; bounding here keeps a hostile `PIC X(4000000000)` from driving a
/// multi-gigabyte allocation, and keeps the `(n)` accumulator from overflowing.
const MAX_PICTURE_SIZE: usize = 65_535;

/// A parsed PICTURE clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picture {
    /// Unsigned numeric-display: `int_digits` positions before the implied
    /// decimal point and `dec_digits` after it. `PIC 9(3)V99` → {3, 2}.
    Numeric { int_digits: usize, dec_digits: usize },
    /// Alphanumeric (`X`): `size` character positions.
    Alphanumeric { size: usize },
    /// Alphabetic (`A`): `size` character positions.
    Alphabetic { size: usize },
}

impl Picture {
    /// Number of storage-bearing character positions.
    pub fn size(&self) -> usize {
        match self {
            Picture::Numeric { int_digits, dec_digits } => int_digits + dec_digits,
            Picture::Alphanumeric { size } | Picture::Alphabetic { size } => *size,
        }
    }

    /// Is this a numeric-display picture?
    pub fn is_numeric(&self) -> bool {
        matches!(self, Picture::Numeric { .. })
    }

    /// Parse a picture string such as `"9(3)V99"`, `"X(20)"`, `"A(5)"`, `"999"`.
    ///
    /// The string is a run of *symbols*, each optionally followed by a `(n)`
    /// repetition count. We expand it to a flat symbol sequence, then classify.
    pub fn parse(pic: &str) -> Result<Picture, RuntimeError> {
        let symbols = expand(pic).ok_or_else(|| RuntimeError::UnsupportedPicture(pic.to_string()))?;
        if symbols.is_empty() {
            return Err(RuntimeError::UnsupportedPicture(pic.to_string()));
        }

        // Classify by the storage-bearing symbols present. Case-insensitive:
        // real source is upper-case, but be forgiving.
        let up: Vec<char> = symbols.iter().map(|c| c.to_ascii_uppercase()).collect();

        let all_x = up.iter().all(|&c| c == 'X');
        let all_a = up.iter().all(|&c| c == 'A');
        let numeric = up.iter().all(|&c| c == '9' || c == 'V');

        if all_x {
            return Ok(Picture::Alphanumeric { size: up.len() });
        }
        if all_a {
            return Ok(Picture::Alphabetic { size: up.len() });
        }
        if numeric {
            // At most one V, splitting integer and fractional digits.
            if up.iter().filter(|&&c| c == 'V').count() > 1 {
                return Err(RuntimeError::UnsupportedPicture(pic.to_string()));
            }
            let v_at = up.iter().position(|&c| c == 'V');
            let (int_digits, dec_digits) = match v_at {
                Some(i) => (i, up.len() - i - 1),
                None => (up.len(), 0),
            };
            return Ok(Picture::Numeric { int_digits, dec_digits });
        }

        // Signed (S), scaling (P), editing symbols, or a mixed class — not v0.1.
        Err(RuntimeError::UnsupportedPicture(pic.to_string()))
    }
}

/// Expand a picture string into its flat sequence of symbols, resolving each
/// `symbol(n)` repetition. Returns `None` on malformed repetition syntax.
fn expand(pic: &str) -> Option<Vec<char>> {
    let chars: Vec<char> = pic.chars().collect();
    let mut out: Vec<char> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        // Bound the total across every path (repeated or literal symbols).
        if out.len() > MAX_PICTURE_SIZE {
            return None;
        }
        let sym = chars[i];
        if sym == '(' || sym == ')' {
            return None; // a paren with no preceding symbol
        }
        i += 1;
        // Optional repetition count `(n)` applies to `sym`.
        if i < chars.len() && chars[i] == '(' {
            let mut j = i + 1;
            let mut n = 0usize;
            let mut saw_digit = false;
            while j < chars.len() && chars[j].is_ascii_digit() {
                // Checked accumulation, capped at MAX_PICTURE_SIZE — a hostile
                // 20-digit count can neither overflow the usize nor slip past
                // the size bound below.
                n = n
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(chars[j] as usize - '0' as usize))
                    .filter(|&v| v <= MAX_PICTURE_SIZE)?;
                saw_digit = true;
                j += 1;
            }
            if !saw_digit || j >= chars.len() || chars[j] != ')' || n == 0 {
                return None;
            }
            // Bound the running total so a picture cannot exceed MAX_PICTURE_SIZE.
            if out.len().saturating_add(n) > MAX_PICTURE_SIZE {
                return None;
            }
            for _ in 0..n {
                out.push(sym);
            }
            i = j + 1;
        } else {
            out.push(sym);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_with_implied_decimal() {
        assert_eq!(Picture::parse("9(3)V99").unwrap(), Picture::Numeric { int_digits: 3, dec_digits: 2 });
        assert_eq!(Picture::parse("9(3)V99").unwrap().size(), 5);
    }

    #[test]
    fn plain_integer_and_expanded_forms_agree() {
        assert_eq!(Picture::parse("999").unwrap(), Picture::Numeric { int_digits: 3, dec_digits: 0 });
        assert_eq!(Picture::parse("9(3)").unwrap(), Picture::parse("999").unwrap());
    }

    #[test]
    fn character_pictures() {
        assert_eq!(Picture::parse("X(20)").unwrap(), Picture::Alphanumeric { size: 20 });
        assert_eq!(Picture::parse("XXX").unwrap().size(), 3);
        assert_eq!(Picture::parse("A(5)").unwrap(), Picture::Alphabetic { size: 5 });
    }

    #[test]
    fn signed_and_editing_are_unsupported_not_wrong() {
        assert!(Picture::parse("S9(4)").is_err());
        assert!(Picture::parse("ZZ9").is_err());
        assert!(Picture::parse("9(3)PPP").is_err());
    }

    #[test]
    fn oversized_repetition_is_rejected_not_allocated() {
        // Would be a 16 GB allocation without the bound — must error instead.
        assert!(Picture::parse("X(4000000000)").is_err());
        // Just over the cap is rejected; the cap itself is accepted.
        assert!(Picture::parse("X(65536)").is_err());
        assert_eq!(Picture::parse("X(65535)").unwrap().size(), 65_535);
    }

    #[test]
    fn overflowing_repetition_count_does_not_panic() {
        // 20-digit count overflows usize — must return an error, never panic.
        assert!(Picture::parse("9(99999999999999999999)").is_err());
    }
}
