//! Query string parsing.
//!
//! Splits `key=value&key2=value2` into a `HashMap<String, String>`.
//! Percent-decoding follows the `application/x-www-form-urlencoded` convention:
//! `+` decodes to a space, `%HH` decodes to the corresponding byte.
//! Keys and values that decode to non-UTF-8 byte sequences are replaced with
//! the Unicode replacement character.

use std::collections::HashMap;

/// Parse a query string (without the leading `?`) into a map.
///
/// Empty string produces an empty map.
/// Keys with no `=` get an empty string value.
/// Duplicate keys keep only the last value.
pub fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if query.is_empty() {
        return map;
    }

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        let key = percent_decode(raw_key);
        if key.is_empty() {
            continue;
        }
        let value = percent_decode(raw_value);
        map.insert(key, value);
    }

    map
}

/// Split a request target into `(path, query_string)`.
///
/// The returned query string does not include the leading `?`.
pub fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    }
}

/// Decode a percent-encoded URL component.
///
/// `+` → space.  `%HH` → byte.  Invalid sequences are passed through as-is.
fn percent_decode(input: &str) -> String {
    // Most components contain no special characters; avoid allocation in the
    // common case by scanning for `%` or `+` first.
    if !input.contains(['%', '+']) {
        return input.to_string();
    }

    let mut output: Vec<u8> = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                output.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (hex_nibble(bytes[i + 1]), hex_nibble(bytes[i + 2]))
                {
                    output.push(hi << 4 | lo);
                    i += 3;
                } else {
                    output.push(b'%');
                    i += 1;
                }
            }
            b => {
                output.push(b);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&output).into_owned()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_key_value_pairs() {
        let map = parse_query_string("name=Adhithya&lang=rust");
        assert_eq!(map["name"], "Adhithya");
        assert_eq!(map["lang"], "rust");
    }

    #[test]
    fn empty_string_returns_empty_map() {
        assert!(parse_query_string("").is_empty());
    }

    #[test]
    fn key_without_equals_gets_empty_value() {
        let map = parse_query_string("flag");
        assert_eq!(map["flag"], "");
    }

    #[test]
    fn plus_decodes_to_space() {
        let map = parse_query_string("msg=hello+world");
        assert_eq!(map["msg"], "hello world");
    }

    #[test]
    fn percent_encoding_is_decoded() {
        let map = parse_query_string("q=hello%20world&emoji=%F0%9F%A6%80");
        assert_eq!(map["q"], "hello world");
        assert_eq!(map["emoji"], "🦀");
    }

    #[test]
    fn empty_key_is_skipped() {
        let map = parse_query_string("=value&key=ok");
        assert!(!map.contains_key(""));
        assert_eq!(map["key"], "ok");
    }

    #[test]
    fn duplicate_key_keeps_last_value() {
        let map = parse_query_string("x=1&x=2");
        assert_eq!(map["x"], "2");
    }

    #[test]
    fn split_target_separates_path_and_query() {
        assert_eq!(split_target("/hello?name=Adhithya"), ("/hello", "name=Adhithya"));
        assert_eq!(split_target("/hello"), ("/hello", ""));
        assert_eq!(split_target("/"), ("/", ""));
    }
}
