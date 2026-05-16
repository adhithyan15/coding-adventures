//! CODE / CHAR / UNICODE / UNICHAR — character code conversion.
//!
//! | function     | input                | output                        |
//! |--------------|----------------------|-------------------------------|
//! | `CODE(s)`    | string               | first char as Latin-1 / lower byte (`0..=255`) |
//! | `CHAR(n)`    | integer 1..=255      | Latin-1 char as 1-char string |
//! | `UNICODE(s)` | string               | first char as Unicode scalar  |
//! | `UNICHAR(n)` | integer 1..=0x10FFFF | Unicode char as 1-char string |
//!
//! Empty input → `BadParameter`. Out-of-range `CHAR` (e.g. 256) is also
//! `BadParameter` (Excel's `#VALUE!`). `UNICHAR` accepts any valid scalar
//! value but rejects surrogates (U+D800..=U+DFFF).

use crate::{iter_character, TextError};
use r_vector::Character;

/// `CODE(text)` — returns the first character as its Latin-1 code (or lower
/// byte for non-Latin-1 chars). For pure ASCII this matches Excel exactly.
/// For non-ASCII we return the lower byte of the Unicode code point as a
/// best-effort match to Excel's legacy behaviour.
pub fn code(s: &str) -> Result<u32, TextError> {
    let c = s.chars().next().ok_or(TextError::BadParameter {
        name: "text",
        value: String::new(),
    })?;
    Ok(c as u32 & 0xff)
}

/// `CHAR(n)` — Latin-1 character. Requires `1 <= n <= 255`.
pub fn char_at(n: i64) -> Result<String, TextError> {
    if !(1..=255).contains(&n) {
        return Err(TextError::BadParameter {
            name: "number",
            value: n.to_string(),
        });
    }
    Ok(char::from(n as u8).to_string())
}

/// `UNICODE(text)` — full Unicode scalar value of the first character.
pub fn unicode(s: &str) -> Result<u32, TextError> {
    let c = s.chars().next().ok_or(TextError::BadParameter {
        name: "text",
        value: String::new(),
    })?;
    Ok(c as u32)
}

/// `UNICHAR(n)` — Unicode scalar value as a single-char string.
///
/// Rejects values outside `1..=0x10FFFF` and surrogate code points.
pub fn unichar(n: i64) -> Result<String, TextError> {
    if !(1..=0x0010_FFFF).contains(&n) {
        return Err(TextError::BadParameter {
            name: "number",
            value: n.to_string(),
        });
    }
    let scalar = n as u32;
    // Reject surrogate range.
    if (0xD800..=0xDFFF).contains(&scalar) {
        return Err(TextError::BadParameter {
            name: "number",
            value: format!("surrogate {n}"),
        });
    }
    char::from_u32(scalar)
        .map(|c| c.to_string())
        .ok_or(TextError::BadParameter {
            name: "number",
            value: n.to_string(),
        })
}

/// Vector `CODE`. NA in / NA out; errors collapse to NA.
pub fn code_vec(x: &Character) -> Vec<Option<u32>> {
    iter_character(x)
        .map(|cell| cell.and_then(|s| code(s).ok()))
        .collect()
}

/// Vector `UNICODE`. NA in / NA out.
pub fn unicode_vec(x: &Character) -> Vec<Option<u32>> {
    iter_character(x)
        .map(|cell| cell.and_then(|s| unicode(s).ok()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_ascii() {
        assert_eq!(code("A").unwrap(), 65);
        assert_eq!(code("a").unwrap(), 97);
        // Only first char matters
        assert_eq!(code("Apple").unwrap(), 65);
        assert_eq!(code(" ").unwrap(), 32);
    }

    #[test]
    fn code_empty_errors() {
        assert!(code("").is_err());
    }

    #[test]
    fn char_at_basic() {
        assert_eq!(char_at(65).unwrap(), "A");
        assert_eq!(char_at(32).unwrap(), " ");
        assert_eq!(char_at(255).unwrap(), char::from(255u8).to_string());
        assert!(char_at(0).is_err());
        assert!(char_at(256).is_err());
        assert!(char_at(-1).is_err());
    }

    #[test]
    fn unicode_handles_full_range() {
        assert_eq!(unicode("A").unwrap(), 65);
        assert_eq!(unicode("漢").unwrap(), 0x6f22);
        assert_eq!(unicode("🙂").unwrap(), 0x1f642);
    }

    #[test]
    fn unicode_empty_errors() {
        assert!(unicode("").is_err());
    }

    #[test]
    fn unichar_full_range() {
        assert_eq!(unichar(65).unwrap(), "A");
        assert_eq!(unichar(0x6f22).unwrap(), "漢");
        assert_eq!(unichar(0x1f642).unwrap(), "🙂");
    }

    #[test]
    fn unichar_rejects_out_of_range() {
        assert!(unichar(0).is_err());
        assert!(unichar(-1).is_err());
        assert!(unichar(0x11_0000).is_err());
        // Surrogates
        assert!(unichar(0xD800).is_err());
        assert!(unichar(0xDFFF).is_err());
    }

    #[test]
    fn vec_variants() {
        let x = Character::from_options(vec![Some("A".into()), None, Some("漢".into())]);
        assert_eq!(code_vec(&x), vec![Some(65), None, Some(0x6f22 & 0xff)]);
        assert_eq!(unicode_vec(&x), vec![Some(65), None, Some(0x6f22)]);
    }
}
