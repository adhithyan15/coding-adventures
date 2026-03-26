//! HTML Entity Decoder
//!
//! GFM requires decoding three forms of HTML character references
//! within text content and link destinations:
//!
//!   Named:   `&amp;`  → `&`    `&lt;` → `<`    `&gt;` → `>`
//!   Decimal: `&#65;`  → `A`    `&#169;` → `©`
//!   Hex:     `&#x41;` → `A`    `&#xA9;` → `©`
//!
//! This module implements decoding for all three forms. Named entities use
//! a lookup table covering all HTML5 named character references (~2125 entries).
//!
//! # Why entity decoding matters
//!
//! GFM says: "An entity reference consists of `&` + any of the valid
//! HTML5 named entities + `;`". The decoded character should appear in the
//! output, not the raw reference. So `&amp;copy;` in source text renders as
//! `©` in HTML (and `&copy;` in HTML source).

use crate::entities_table;

/// Look up a named HTML entity and return its Unicode value.
///
/// The lookup table covers all ~2125 HTML5 named character references.
/// Returns `None` for unknown entity names.
///
/// # Examples
///
/// ```
/// use gfm_parser::entities::lookup_named_entity;
/// assert_eq!(lookup_named_entity("amp"), Some("&"));
/// assert_eq!(lookup_named_entity("lt"), Some("<"));
/// assert_eq!(lookup_named_entity("notavalidentity"), None);
/// ```
pub fn lookup_named_entity(name: &str) -> Option<&'static str> {
    // Binary search through the sorted entity table.
    // The table is sorted by entity name for O(log n) lookup.
    let idx = entities_table::NAMED_ENTITIES.binary_search_by_key(&name, |&(k, _)| k);
    idx.ok().map(|i| entities_table::NAMED_ENTITIES[i].1)
}

/// Decode a single HTML character reference (the `&...;` token including ampersand and semicolon).
///
/// Handles all three forms:
///   - Named:   `&amp;` → `&`
///   - Decimal: `&#65;` → `A`
///   - Hex:     `&#x41;` → `A`
///
/// Returns the decoded character as a `String`, or the original reference unchanged
/// if it is not recognised.
///
/// # Examples
///
/// ```
/// use gfm_parser::entities::decode_entity;
/// assert_eq!(decode_entity("&amp;"), "&");
/// assert_eq!(decode_entity("&#65;"), "A");
/// assert_eq!(decode_entity("&#x41;"), "A");
/// assert_eq!(decode_entity("&notvalid;"), "&notvalid;");
/// ```
pub fn decode_entity(entity: &str) -> String {
    if !entity.starts_with('&') || !entity.ends_with(';') {
        return entity.to_string();
    }

    let inner = &entity[1..entity.len() - 1];

    if let Some(rest) = inner.strip_prefix('#') {
        // Numeric entity
        let (base, digits) = if rest.starts_with('x') || rest.starts_with('X') {
            (16u32, &rest[1..])
        } else {
            (10u32, rest)
        };

        if let Ok(code_point) = u32::from_str_radix(digits, base) {
            // GFM: numeric character references that decode to NUL (U+0000)
            // are replaced with the Unicode replacement character U+FFFD.
            // The spec also excludes lone surrogates and values > U+10FFFF.
            let ch = if code_point == 0 {
                '\u{FFFD}'
            } else {
                char::from_u32(code_point).unwrap_or('\u{FFFD}')
            };
            return ch.to_string();
        }
        return entity.to_string();
    }

    // Named entity lookup
    if let Some(value) = lookup_named_entity(inner) {
        return value.to_string();
    }

    entity.to_string()
}

/// Decode all HTML character references in a string.
///
/// Scans for `&...;` patterns and replaces each recognised reference
/// with its decoded character. Unrecognised references are left as-is.
///
/// # Examples
///
/// ```
/// use gfm_parser::entities::decode_entities;
/// assert_eq!(decode_entities("Tom &amp; Jerry"), "Tom & Jerry");
/// assert_eq!(decode_entities("&lt;p&gt;hello&lt;/p&gt;"), "<p>hello</p>");
/// assert_eq!(decode_entities("no entities here"), "no entities here");
/// ```
pub fn decode_entities(text: &str) -> String {
    // Fast path: no `&` means no entities
    if !text.contains('&') {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'&' {
            // Find next `&` to bulk-copy
            let start = i;
            while i < bytes.len() && bytes[i] != b'&' {
                i += 1;
            }
            result.push_str(&text[start..i]);
            continue;
        }

        // Try to match `&...;`
        let amp_pos = i;
        i += 1; // skip `&`

        if i >= bytes.len() {
            result.push('&');
            continue;
        }

        // Collect up to 33 characters looking for `;`
        let mut j = i;
        let limit = (i + 33).min(bytes.len());
        while j < limit && bytes[j] != b';' && bytes[j] != b'&' && bytes[j] != b'\n' {
            j += 1;
        }

        if j < bytes.len() && bytes[j] == b';' {
            let entity_str = &text[amp_pos..j + 1];
            let decoded = decode_entity(entity_str);
            result.push_str(&decoded);
            i = j + 1;
        } else {
            result.push('&');
            // i stays at next char after &
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_entity_amp() {
        assert_eq!(decode_entity("&amp;"), "&");
    }

    #[test]
    fn test_named_entity_lt_gt() {
        assert_eq!(decode_entity("&lt;"), "<");
        assert_eq!(decode_entity("&gt;"), ">");
    }

    #[test]
    fn test_named_entity_quot() {
        assert_eq!(decode_entity("&quot;"), "\"");
    }

    #[test]
    fn test_decimal_entity() {
        assert_eq!(decode_entity("&#65;"), "A");
        assert_eq!(decode_entity("&#169;"), "©");
    }

    #[test]
    fn test_hex_entity() {
        assert_eq!(decode_entity("&#x41;"), "A");
        assert_eq!(decode_entity("&#X41;"), "A");
        assert_eq!(decode_entity("&#xA9;"), "©");
    }

    #[test]
    fn test_null_entity_becomes_replacement_char() {
        assert_eq!(decode_entity("&#0;"), "\u{FFFD}");
    }

    #[test]
    fn test_unknown_entity_unchanged() {
        assert_eq!(decode_entity("&notvalid;"), "&notvalid;");
    }

    #[test]
    fn test_decode_entities_string() {
        assert_eq!(decode_entities("Tom &amp; Jerry"), "Tom & Jerry");
        assert_eq!(decode_entities("&lt;p&gt;"), "<p>");
        assert_eq!(decode_entities("no entities"), "no entities");
    }

    #[test]
    fn test_lookup_named_entity() {
        assert_eq!(lookup_named_entity("amp"), Some("&"));
        assert_eq!(lookup_named_entity("copy"), Some("©"));
        assert_eq!(lookup_named_entity("notexist"), None);
    }
}
