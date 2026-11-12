//! # TFV v1: the line format corpora travel in (TF01 §2)
//!
//! Corpora are written as JSON for people to read, then compiled into TFV
//! for harnesses to read. TFV exists because the bottom layer may not depend
//! on a third-party JSON parser, and many languages have none built in.
//!
//! ## One record per line
//!
//! ```text
//!   tfv 1
//!   deep_equal/int-equal R4 Kop S646565705f657175616c Kactual I5 Kexpected I5 Kverdict S70617373
//!   └─── record id ────┘ └┘ └┘ └── value ───────────┘
//!                       field  field name
//!                       count
//! ```
//!
//! Values are written in *prefix* notation: a tag character, then its body,
//! and for containers a count followed by that many values.
//!
//! | Token          | Value                      | Example          |
//! |----------------|----------------------------|------------------|
//! | `N`            | null                       | `N`              |
//! | `T` / `F`      | true / false               | `T`              |
//! | `I<int>`       | integer, any size          | `I-12`           |
//! | `D<16 hex>`    | f64, its exact IEEE bits   | `D8000000000000000` (that is -0.0) |
//! | `X<hex>`       | bytes                      | `X00ff`          |
//! | `S<hex>`       | text, as hex of its UTF-8  | `S6869` ("hi")   |
//! | `L<n> v…`      | list of n values           | `L2 I1 I2`       |
//! | `M<n> k v…`    | map of n key/value pairs   | `M1 S61 T`       |
//!
//! Text is hex-encoded so a value never contains a space or a newline, which
//! keeps splitting on single spaces correct.
//!
//! ## Strictness
//!
//! There is exactly one way to write each value, and the parser rejects
//! everything else: uppercase hex, `I007`, `I-0`, two spaces in a row, counts
//! that disagree with what follows. A lenient parser would let two lanes read
//! one corpus differently, and the corpus is the thing every lane agrees on.

/// TFV's first line, exactly.
pub const HEADER: &str = "tfv 1";

/// How deeply lists and maps may nest. The outermost container is depth 1;
/// a 65th level is rejected. The limit keeps the recursive parser's stack
/// use bounded whatever the input.
pub const MAX_DEPTH: usize = 64;

/// How many digits an integer may have, not counting its sign. Arbitrary
/// precision still, but bounded, so that no lane meets a limit of its own
/// first: Python, for one, refuses to convert more than 4300 digits.
pub const MAX_INT_DIGITS: usize = 1000;

/// One TFV value.
///
/// There is deliberately no `PartialEq`: the only equality the harness
/// knows is [`values_equal`], whose rules (NaN, -0, map order) are the
/// spec's, not Rust's.
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    /// An integer of any size, held as its canonical decimal text: an
    /// optional `-`, no leading zeros, never `-0`. Canonical text means two
    /// integers are equal exactly when their texts are.
    Int(String),
    /// The raw IEEE-754 bit pattern, so `-0.0` and every NaN survive.
    F64(u64),
    Bytes(Vec<u8>),
    Text(String),
    List(Vec<Value>),
    /// Key/value pairs in written order. No two keys are deep-equal.
    Map(Vec<(Value, Value)>),
}

impl Value {
    /// An integer value from a `u64`.
    pub fn from_u64(n: u64) -> Value {
        Value::Int(n.to_string())
    }

    /// This integer as a `u64`, if it is one and fits.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Int(text) if !text.starts_with('-') => text.parse().ok(),
            _ => None,
        }
    }

    /// This text value's string.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    /// This bytes value's bytes.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// This list value's items.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }
}

/// One line of a TFV file: an id and its named fields, in written order.
#[derive(Clone, Debug)]
pub struct Record {
    pub id: String,
    pub fields: Vec<(String, Value)>,
}

impl Record {
    /// The value of the field called `name`.
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }
}

/// Why a TFV text was rejected, and on which line (1-based).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TfvError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for TfvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for TfvError {}

fn reject<T>(line: usize, message: impl Into<String>) -> Result<T, TfvError> {
    Err(TfvError {
        line,
        message: message.into(),
    })
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a whole TFV file, or say which line broke which rule.
///
/// ```text
///   "tfv 1\nr R1 Kv I5\n"   → [Record { id: "r", fields: [("v", Int("5"))] }]
///   "tfv 1\nr R1 Kv I05\n"  → Err(line 2: malformed token "I05")
/// ```
///
/// A final newline is optional. Blank lines and lines starting with `#`
/// are skipped. Record ids must be unique within the file.
pub fn parse_file(text: &str) -> Result<Vec<Record>, TfvError> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    if lines.first() != Some(&HEADER) {
        return reject(1, "first line must be exactly 'tfv 1'");
    }
    let mut records: Vec<Record> = Vec::new();
    // A set, not a scan of `records`: a table of 100,000 rows (VER04j WX-E) made
    // the scan quadratic (about 5 * 10^9 comparisons).
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (index, line) in lines.iter().enumerate().skip(1) {
        let number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let record = parse_record(line, number)?;
        if !seen_ids.insert(record.id.clone()) {
            return reject(number, format!("duplicate record id {:?}", record.id));
        }
        records.push(record);
    }
    Ok(records)
}

/// `<id> R<n> K<field> <value> ... ` with exactly `n` fields and nothing after.
fn parse_record(line: &str, number: usize) -> Result<Record, TfvError> {
    if line.contains('\r') {
        return reject(number, "carriage return");
    }
    // Splitting on single spaces turns "a  b" into ["a", "", "b"], so an
    // empty token is how a double, leading or trailing space shows up.
    let tokens: Vec<&str> = line.split(' ').collect();
    if tokens.iter().any(|token| token.is_empty()) {
        return reject(number, "empty token (double, leading or trailing space)");
    }
    if tokens.len() < 2 || !is_record_id(tokens[0]) {
        return reject(number, "bad record id");
    }
    let count = match tokens[1].strip_prefix('R') {
        Some(body) if is_count(body) => parse_count(body, number)?,
        _ => return reject(number, "expected R<count>"),
    };
    let mut position = 2;
    let mut fields: Vec<(String, Value)> = Vec::new();
    for _ in 0..count {
        let Some(token) = tokens.get(position) else {
            return reject(number, "record ends early");
        };
        let name = match token.strip_prefix('K') {
            Some(name) if is_field_name(name) => name,
            _ => return reject(number, format!("expected K<field>, got {token:?}")),
        };
        if fields.iter().any(|(seen, _)| seen == name) {
            return reject(number, format!("duplicate field {name:?}"));
        }
        let (value, next) = parse_value(&tokens, position + 1, number, 0)?;
        fields.push((name.to_owned(), value));
        position = next;
    }
    if position != tokens.len() {
        return reject(number, "trailing tokens");
    }
    Ok(Record {
        id: tokens[0].to_owned(),
        fields,
    })
}

/// Parse the value starting at `tokens[position]`, returning it and the
/// position just past it. `depth` is how many containers enclose it.
fn parse_value(
    tokens: &[&str],
    position: usize,
    number: usize,
    depth: usize,
) -> Result<(Value, usize), TfvError> {
    let Some(token) = tokens.get(position) else {
        return reject(number, "value missing");
    };
    // Split off the tag as a *character*: slicing at byte 1 would panic on a
    // token that starts with a multi-byte character.
    let mut chars = token.chars();
    let tag = chars.next();
    let body = chars.as_str();
    let next = position + 1;
    let malformed = || reject(number, format!("malformed token {token:?}"));
    match tag {
        Some('N') if body.is_empty() => Ok((Value::Null, next)),
        Some('T') if body.is_empty() => Ok((Value::Bool(true), next)),
        Some('F') if body.is_empty() => Ok((Value::Bool(false), next)),
        Some('I') if is_canonical_int(body) => Ok((Value::Int(body.to_owned()), next)),
        Some('D') if body.len() == 16 && is_lower_hex(body) => {
            // 16 lowercase hex digits always fit a u64.
            let bits = u64::from_str_radix(body, 16).map_err(|_| TfvError {
                line: number,
                message: format!("malformed token {token:?}"),
            })?;
            Ok((Value::F64(bits), next))
        }
        Some('X') if is_even_lower_hex(body) => Ok((Value::Bytes(decode_hex(body)), next)),
        Some('S') if is_even_lower_hex(body) => match String::from_utf8(decode_hex(body)) {
            Ok(text) => Ok((Value::Text(text), next)),
            Err(_) => reject(number, "text is not valid UTF-8"),
        },
        Some(tag @ ('L' | 'M')) if is_count(body) => {
            if depth >= MAX_DEPTH {
                return reject(
                    number,
                    format!("lists and maps nest at most {MAX_DEPTH} deep"),
                );
            }
            let count = parse_count(body, number)?;
            let mut position = next;
            if tag == 'L' {
                let mut items = Vec::new();
                for _ in 0..count {
                    let (item, after) = parse_value(tokens, position, number, depth + 1)?;
                    items.push(item);
                    position = after;
                }
                return Ok((Value::List(items), position));
            }
            let mut pairs: Vec<(Value, Value)> = Vec::new();
            for _ in 0..count {
                let (key, after_key) = parse_value(tokens, position, number, depth + 1)?;
                let (item, after_item) = parse_value(tokens, after_key, number, depth + 1)?;
                if pairs.iter().any(|(seen, _)| values_equal(seen, &key)) {
                    return reject(number, "map has two deep-equal keys");
                }
                pairs.push((key, item));
                position = after_item;
            }
            Ok((Value::Map(pairs), position))
        }
        _ => malformed(),
    }
}

/// A count token's body: `0` or digits without a leading zero. A count
/// too large for `usize` cannot be honest (there are not that many tokens
/// on the line), so it is rejected rather than wrapped.
fn parse_count(body: &str, number: usize) -> Result<usize, TfvError> {
    body.parse().or_else(|_| reject(number, "count too large"))
}

// The character classes, spelled out rather than taken from a regex crate.

fn is_record_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'/' | b'-'))
}

fn is_field_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z'))
        && bytes.all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

fn is_count(s: &str) -> bool {
    s == "0"
        || (s.starts_with(|c: char| ('1'..='9').contains(&c))
            && s.bytes().all(|b| b.is_ascii_digit()))
}

/// `-?(0|[1-9][0-9]*)` with at most [`MAX_INT_DIGITS`] digits, never `-0`.
fn is_canonical_int(s: &str) -> bool {
    let digits = s.strip_prefix('-').unwrap_or(s);
    digits.len() <= MAX_INT_DIGITS && is_count(digits) && s != "-0"
}

fn is_lower_hex(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_even_lower_hex(s: &str) -> bool {
    s.len().is_multiple_of(2) && is_lower_hex(s)
}

/// Decode hex already checked by [`is_even_lower_hex`].
fn decode_hex(s: &str) -> Vec<u8> {
    fn nibble(b: u8) -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            _ => b - b'a' + 10,
        }
    }
    let (pairs, _) = s.as_bytes().as_chunks::<2>();
    pairs
        .iter()
        .map(|&[high, low]| (nibble(high) << 4) | nibble(low))
        .collect()
}

// ---------------------------------------------------------------------------
// Writing a value back out (for failure messages)
// ---------------------------------------------------------------------------

/// A value in TFV token form, e.g. `L2 I1 S61`.
///
/// Failure messages use it so "expected" and "got" read in the same
/// notation as the corpus they came from.
pub fn encode(value: &Value) -> String {
    let mut out = String::new();
    encode_into(value, &mut out);
    out
}

fn encode_into(value: &Value, out: &mut String) {
    use std::fmt::Write as _;
    if !out.is_empty() {
        out.push(' ');
    }
    // Writing to a String cannot fail.
    let _ = match value {
        Value::Null => write!(out, "N"),
        Value::Bool(true) => write!(out, "T"),
        Value::Bool(false) => write!(out, "F"),
        Value::Int(text) => write!(out, "I{text}"),
        Value::F64(bits) => write!(out, "D{bits:016x}"),
        Value::Bytes(bytes) => write!(out, "X{}", hex(bytes)),
        Value::Text(text) => write!(out, "S{}", hex(text.as_bytes())),
        Value::List(items) => {
            let _ = write!(out, "L{}", items.len());
            for item in items {
                encode_into(item, out);
            }
            Ok(())
        }
        Value::Map(pairs) => {
            let _ = write!(out, "M{}", pairs.len());
            for (key, item) in pairs {
                encode_into(key, out);
                encode_into(item, out);
            }
            Ok(())
        }
    };
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Deep equality (TF01 §3.1)
// ---------------------------------------------------------------------------

/// The quiet NaN every NaN is compared as.
const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

/// Every NaN becomes one NaN; every other pattern is left alone.
///
/// A NaN has all exponent bits set and a non-zero mantissa:
///
/// ```text
///   0x7ff8000000000000  quiet NaN        → itself
///   0x7ff0000000000001  signalling NaN   → 0x7ff8000000000000
///   0x7ff0000000000000  +infinity        → itself (mantissa is zero)
///   0x8000000000000000  -0.0             → itself, so -0 ≠ +0
/// ```
fn canonical_f64(bits: u64) -> u64 {
    let exponent = (bits >> 52) & 0x7ff;
    let mantissa = bits & ((1 << 52) - 1);
    if exponent == 0x7ff && mantissa != 0 {
        CANONICAL_NAN
    } else {
        bits
    }
}

/// Are two values deep-equal under TF01 §3.1?
///
/// | Kinds         | Equal when                                          |
/// |---------------|-----------------------------------------------------|
/// | null, bool    | same value                                          |
/// | int           | same integer (canonical text makes this text ==)    |
/// | f64           | same bits after canonicalising NaN                  |
/// | bytes, text   | same sequence; bytes never equal text               |
/// | list          | same length, pairwise equal                         |
/// | map           | same size, each key matched by an equal key whose value is equal |
/// | other kinds   | never: `I1` is not `D3ff0000000000000`              |
pub fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::F64(x), Value::F64(y)) => canonical_f64(*x) == canonical_f64(*y),
        (Value::Bytes(x), Value::Bytes(y)) => x == y,
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::List(xs), Value::List(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| values_equal(x, y))
        }
        (Value::Map(xs), Value::Map(ys)) => {
            xs.len() == ys.len()
                && xs.iter().all(|(key, item)| {
                    let mut matches = ys.iter().filter(|(other, _)| values_equal(key, other));
                    match (matches.next(), matches.next()) {
                        (Some((_, other_item)), None) => values_equal(item, other_item),
                        _ => false,
                    }
                })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_value(text: &str) -> Result<Value, TfvError> {
        let file = format!("tfv 1\nr R1 Kv {text}\n");
        parse_file(&file).map(|mut records| records.remove(0).fields.remove(0).1)
    }

    #[test]
    fn every_kind_round_trips_through_encode() {
        for text in [
            "N",
            "T",
            "F",
            "I0",
            "I-12",
            "I123456789012345678901234567890",
            "D8000000000000000",
            "X",
            "X00ff",
            "S",
            "S6869",
            "L0",
            "L2 I1 L1 N",
            "M2 S61 T I1 X00",
        ] {
            let value = one_value(text).unwrap_or_else(|e| panic!("{text}: {e}"));
            assert_eq!(encode(&value), text);
        }
    }

    #[test]
    fn comments_blank_lines_and_a_missing_final_newline_are_fine() {
        let records = parse_file("tfv 1\n# note\n\na R0\nb R1 Kx T").unwrap();
        assert_eq!(records.len(), 2);
        assert!(records[1].field("x").is_some());
        assert!(records[1].field("y").is_none());
    }

    #[test]
    fn a_hundred_thousand_rows_parse_and_a_duplicate_is_still_the_line_that_repeats_it() {
        // Linear duplicate detection (VER04j WX-E: tables of up to 100,000 rows).
        let mut text = String::from("tfv 1\n");
        for i in 0..100_000 {
            text.push_str(&format!("row/{i} R1 Kx I{i}\n"));
        }
        let started = std::time::Instant::now();
        let records = parse_file(&text).unwrap();
        assert_eq!(records.len(), 100_000);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(20),
            "parsing 100,000 rows took {:?}",
            started.elapsed()
        );
        text.push_str("row/77 R0\n");
        let error = parse_file(&text).unwrap_err();
        assert_eq!(error.line, 100_002);
        assert!(error.message.contains("duplicate record id"));
    }

    #[test]
    fn errors_name_their_line() {
        let error = parse_file("tfv 1\na R0\nb R0 junk\n").unwrap_err();
        assert_eq!(error.line, 3);
        assert_eq!(error.to_string(), "line 3: trailing tokens");
    }

    #[test]
    fn rejections_the_corpus_does_not_cover() {
        for bad in [
            "tfv 1\nr R1 Kv I-\n",
            "tfv 1\nr R1 Kv L01 N\n",
            "tfv 1\nr R99999999999999999999999 Kv N\n",
            "tfv 1\nr R1 Kv NN\n",
            "tfv 1\nr\n",
            "tfv 1\nr Q0\n",
            "tfv 1\nr R1 Kv M1 N\n",
            "tfv 1\nr R1 Kv Dffffffffffffffffff\n",
        ] {
            assert!(parse_file(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn integer_digit_limit_is_exact() {
        let at_limit = format!("I-9{}", "0".repeat(MAX_INT_DIGITS - 1));
        let past_limit = format!("I9{}", "0".repeat(MAX_INT_DIGITS));
        assert!(one_value(&at_limit).is_ok());
        assert!(one_value(&past_limit).is_err());
    }

    #[test]
    fn nesting_limit_is_exact() {
        let at_limit = format!("{}N", "L1 ".repeat(MAX_DEPTH));
        let past_limit = format!("{}N", "L1 ".repeat(MAX_DEPTH + 1));
        assert!(one_value(&at_limit).is_ok());
        assert!(one_value(&past_limit).is_err());
    }

    #[test]
    fn accessors_check_the_kind() {
        assert_eq!(Value::from_u64(u64::MAX).as_u64(), Some(u64::MAX));
        assert_eq!(Value::Int("-1".into()).as_u64(), None);
        assert_eq!(Value::Int("18446744073709551616".into()).as_u64(), None);
        assert_eq!(Value::Null.as_u64(), None);
        assert_eq!(Value::Text("a".into()).as_text(), Some("a"));
        assert_eq!(Value::Null.as_text(), None);
        assert_eq!(Value::Bytes(vec![1]).as_bytes(), Some(&[1u8][..]));
        assert_eq!(Value::Null.as_bytes(), None);
        assert!(Value::List(vec![]).as_list().is_some());
        assert!(Value::Null.as_list().is_none());
    }

    #[test]
    fn infinity_is_not_a_nan() {
        assert!(!values_equal(
            &Value::F64(0x7ff0_0000_0000_0000),
            &Value::F64(CANONICAL_NAN)
        ));
    }
}
