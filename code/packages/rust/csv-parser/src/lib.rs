//! # CSV Parser
//!
//! A hand-rolled **state machine** CSV parser following RFC 4180 semantics.
//!
//! CSV (Comma-Separated Values) is deceptively simple to look at but tricky
//! to parse correctly. The core difficulty is that CSV is **context-sensitive**:
//! the meaning of a comma character depends on whether you are currently inside
//! a quoted field or not. You cannot know ahead of time — you must track state.
//!
//! Think of it like reading a book with footnotes: when you see an opening
//! quotation mark in the text, you enter "quoted mode" and must treat everything
//! differently (including commas) until you see the closing quotation mark.
//!
//! # Why a state machine?
//!
//! A state machine is the natural fit here. At any point in the input, the
//! parser is in exactly one state, and each character causes either:
//! - A transition to a new state, OR
//! - An action (append to current field, emit a completed field), OR
//! - Both simultaneously.
//!
//! This avoids lookahead (checking future characters before deciding what to do),
//! makes the code easy to audit against the spec, and produces clean error messages.
//!
//! # The four states
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                                                                         │
//! │  FIELD_START                                                            │
//! │      │                                                                  │
//! │      ├─── '"' ──────────────────────────► IN_QUOTED_FIELD              │
//! │      │                                         │                       │
//! │      │                              other char ┤ append to buffer      │
//! │      │                                         │                       │
//! │      │                              '"' ───────► IN_QUOTED_MAYBE_END   │
//! │      │                                                   │             │
//! │      │                              '"' again ───────────┤             │
//! │      │                              (append '"', back to IN_QUOTED)   │
//! │      │                                                   │             │
//! │      │                              COMMA/NEWLINE/EOF ───► end field   │
//! │      │                                                                 │
//! │      └─── other ────────────────────────► IN_UNQUOTED_FIELD           │
//! │                                                   │                   │
//! │                                      other char ──┤ append to buffer  │
//! │                                                   │                   │
//! │                                      COMMA ───────► end field, next   │
//! │                                      NEWLINE ─────► end field, new row│
//! │                                      EOF ─────────► end field, done   │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Grammar (EBNF)
//!
//! ```text
//! file         = [header] { record }
//! header       = record                   # first row is the header
//! record       = field { COMMA field } (NEWLINE | EOF)
//! field        = quoted | unquoted
//! quoted       = '"' { QCHAR | COMMA | NEWLINE | '""' } '"'
//! unquoted     = { UCHAR }               # may be empty string ""
//!
//! COMMA        = ","                     # default delimiter (configurable)
//! NEWLINE      = "\r\n" | "\n" | "\r"
//! ESCAPED_QUOTE = '""'                  # two double-quotes inside a quoted field
//! QCHAR        = any char except '"'
//! UCHAR        = any char except COMMA, '"', NEWLINE, EOF
//! ```
//!
//! # Examples
//!
//! ## Simple table
//!
//! ```rust
//! use coding_adventures_csv_parser::parse_csv;
//!
//! let csv = "name,age,city\nAlice,30,New York\nBob,25,London\n";
//! let rows = parse_csv(csv).unwrap();
//!
//! assert_eq!(rows[0]["name"], "Alice");
//! assert_eq!(rows[0]["age"], "30");   // note: string, not integer!
//! assert_eq!(rows[1]["city"], "London");
//! ```
//!
//! ## Quoted field with embedded comma
//!
//! ```rust
//! use coding_adventures_csv_parser::parse_csv;
//!
//! let csv = "product,description\nWidget,\"A small, round widget\"\n";
//! let rows = parse_csv(csv).unwrap();
//! assert_eq!(rows[0]["description"], "A small, round widget");
//! ```
//!
//! ## Custom delimiter (TSV)
//!
//! ```rust
//! use coding_adventures_csv_parser::parse_csv_with_delimiter;
//!
//! let tsv = "name\tage\nAlice\t30\n";
//! let rows = parse_csv_with_delimiter(tsv, '\t').unwrap();
//! assert_eq!(rows[0]["name"], "Alice");
//! ```

use std::collections::HashMap;

// ===========================================================================
// Error type
// ===========================================================================

/// Errors that can occur during CSV parsing.
///
/// Currently the only recoverable error is an unclosed quoted field, which
/// happens when the input ends while we are still inside a `"..."` block.
/// All other anomalies (ragged rows, empty files) are handled silently per
/// the spec: pad short rows with `""`, truncate long rows.
#[derive(Debug, PartialEq)]
pub enum CsvError {
    /// An opening `"` was never closed before EOF.
    ///
    /// Example: the input `name,value\n1,"unclosed` ends with the parser
    /// still inside a quoted field. There is no valid way to interpret the
    /// partial field, so we return this error.
    UnclosedQuote,
}

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsvError::UnclosedQuote => write!(
                f,
                "unclosed quoted field: EOF reached inside a quoted field"
            ),
        }
    }
}

impl std::error::Error for CsvError {}

// ===========================================================================
// State machine states
// ===========================================================================

/// The four states of the CSV parse state machine.
///
/// Think of these like the "modes" you can be in while reading the file.
/// At any moment, exactly one of these is active:
///
/// | State              | Meaning                                           |
/// |--------------------|---------------------------------------------------|
/// | FieldStart         | Between fields; about to start a new field        |
/// | InUnquotedField    | Reading plain text (no quotes involved)           |
/// | InQuotedField      | Inside `"..."`, commas and newlines are literal   |
/// | InQuotedMaybeEnd   | Saw `"` inside a quoted field; waiting for next   |
///
/// The `InQuotedMaybeEnd` state is the trickiest. When we see `"` inside a
/// quoted field, we do not immediately end the field because that `"` might
/// be the first half of an escape sequence `""` (which means a literal `"`).
/// We have to wait for the next character to decide:
///
/// - Next char is `"` → it's an escape: append `"` and go back to `InQuotedField`
/// - Next char is the delimiter, newline, or EOF → the quote was a closing quote
/// - Next char is anything else → technically malformed; we treat it as end-of-quote
///   followed by the unexpected character (lenient mode — many real CSV files do this)
#[derive(Debug, PartialEq, Clone)]
enum ParseState {
    /// Just started (or just finished) a field. The next character will tell
    /// us whether this field is quoted (`"`) or unquoted (anything else).
    FieldStart,

    /// We are accumulating characters for an unquoted (plain text) field.
    /// We stay here until we hit the delimiter, a newline, or EOF.
    InUnquotedField,

    /// We are inside a `"..."` quoted field. Only the `"` character is special
    /// here; commas and newlines are treated as literal characters and appended
    /// to the current field value.
    InQuotedField,

    /// We just saw a `"` while in `InQuotedField`. We cannot decide yet whether
    /// this is an escaped quote `""` or the closing quote of the field. We wait
    /// for one more character.
    InQuotedMaybeEnd,
}

// ===========================================================================
// Public API
// ===========================================================================

/// Parse CSV text using the default comma delimiter.
///
/// # Arguments
///
/// - `source` — the full CSV text as a UTF-8 string slice. May be empty.
///
/// # Returns
///
/// - `Ok(rows)` — a `Vec<HashMap<String, String>>` where each map represents
///   one data row (not counting the header row). Keys are column names from
///   the header; values are all strings.
/// - `Err(CsvError::UnclosedQuote)` — if a quoted field is not closed before EOF.
///
/// # Behaviour
///
/// - The **first row** is always treated as the header. It defines the column names.
/// - All values are returned as strings — no type coercion is performed.
/// - Empty unquoted fields (two consecutive delimiters) produce the empty string `""`.
/// - Rows shorter than the header are padded with `""` for missing columns.
/// - Rows longer than the header have extra fields silently discarded.
/// - Trailing newlines are handled correctly (the final record is not duplicated).
///
/// # Examples
///
/// ```rust
/// use coding_adventures_csv_parser::parse_csv;
///
/// let csv = "name,age\nAlice,30\nBob,25\n";
/// let rows = parse_csv(csv).unwrap();
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0]["name"], "Alice");
/// assert_eq!(rows[1]["age"], "25");
/// ```
pub fn parse_csv(source: &str) -> Result<Vec<HashMap<String, String>>, CsvError> {
    parse_csv_with_delimiter(source, ',')
}

/// Parse CSV text with a configurable field delimiter.
///
/// Identical to [`parse_csv`], but accepts any single character as the
/// field delimiter instead of the default comma.
///
/// Common alternatives:
/// - `'\t'` — Tab-Separated Values (TSV)
/// - `';'` — European CSV (where `,` is used as the decimal separator)
/// - `'|'` — Pipe-separated (used in some database exports)
///
/// # Arguments
///
/// - `source` — the full CSV text as a UTF-8 string slice.
/// - `delimiter` — the field separator character. Must not be `"` (the quote
///   character), as that would make the format ambiguous.
///
/// # Examples
///
/// ```rust
/// use coding_adventures_csv_parser::parse_csv_with_delimiter;
///
/// let tsv = "name\tage\nAlice\t30\n";
/// let rows = parse_csv_with_delimiter(tsv, '\t').unwrap();
/// assert_eq!(rows[0]["name"], "Alice");
/// assert_eq!(rows[0]["age"], "30");
/// ```
pub fn parse_csv_with_delimiter(
    source: &str,
    delimiter: char,
) -> Result<Vec<HashMap<String, String>>, CsvError> {
    // -----------------------------------------------------------------------
    // Phase 1: Tokenise the source into raw rows.
    //
    // We walk the input character-by-character, using a state machine to
    // decide what each character means in context. The result is a
    // Vec<Vec<String>>: a list of rows, each row being a list of field strings.
    //
    // We do NOT build the header->value maps yet — that is Phase 2. Keeping
    // the two phases separate makes the code easier to understand and test.
    // -----------------------------------------------------------------------
    let rows = tokenise_rows(source, delimiter)?;

    // -----------------------------------------------------------------------
    // Phase 2: Apply the header to build the final maps.
    //
    // The first row is the header. Every subsequent row is a data row.
    // We zip each data row's fields with the header's column names.
    // -----------------------------------------------------------------------
    if rows.is_empty() {
        // Empty file: no header, no data.
        return Ok(vec![]);
    }

    let header = &rows[0];
    let data_rows = &rows[1..];

    if data_rows.is_empty() {
        // Header-only file: the spec says return an empty list.
        return Ok(vec![]);
    }

    let result = data_rows
        .iter()
        .map(|row| build_row_map(header, row))
        .collect();

    Ok(result)
}

// ===========================================================================
// Internal: tokenise_rows
// ===========================================================================

/// Walk the CSV source character-by-character using the state machine and
/// return a raw `Vec<Vec<String>>` — rows of field strings.
///
/// This function contains all the state machine logic. It does NOT know about
/// headers or column names; it just splits the input into rows and fields.
///
/// # State transition table
///
/// ```text
/// ┌──────────────────────┬──────────────────┬───────────────────────────────┐
/// │ Current State        │ Character        │ Action                        │
/// ├──────────────────────┼──────────────────┼───────────────────────────────┤
/// │ FieldStart           │ '"'              │ → InQuotedField               │
/// │ FieldStart           │ delimiter        │ push "" field, stay FieldStart│
/// │ FieldStart           │ '\n'/'\r'        │ end of row, new row starts    │
/// │ FieldStart           │ EOF              │ end of input                  │
/// │ FieldStart           │ other            │ append to buf, → InUnquoted   │
/// ├──────────────────────┼──────────────────┼───────────────────────────────┤
/// │ InUnquotedField      │ delimiter        │ push buf as field, FieldStart │
/// │ InUnquotedField      │ '\n'/'\r'        │ push buf as field, new row    │
/// │ InUnquotedField      │ EOF              │ push buf as field, done       │
/// │ InUnquotedField      │ other            │ append to buf                 │
/// ├──────────────────────┼──────────────────┼───────────────────────────────┤
/// │ InQuotedField        │ '"'              │ → InQuotedMaybeEnd            │
/// │ InQuotedField        │ other            │ append to buf                 │
/// ├──────────────────────┼──────────────────┼───────────────────────────────┤
/// │ InQuotedMaybeEnd     │ '"'              │ append '"' to buf, ←InQuoted  │
/// │ InQuotedMaybeEnd     │ delimiter        │ push buf as field, FieldStart │
/// │ InQuotedMaybeEnd     │ '\n'/'\r'        │ push buf as field, new row    │
/// │ InQuotedMaybeEnd     │ EOF              │ push buf as field, done       │
/// │ InQuotedMaybeEnd     │ other (lenient)  │ append to buf, ← InUnquoted   │
/// └──────────────────────┴──────────────────┴───────────────────────────────┘
/// ```
fn tokenise_rows(source: &str, delimiter: char) -> Result<Vec<Vec<String>>, CsvError> {
    // The finished rows accumulated so far.
    let mut rows: Vec<Vec<String>> = Vec::new();

    // The fields accumulated for the current (in-progress) row.
    let mut current_row: Vec<String> = Vec::new();

    // The characters accumulated for the current (in-progress) field.
    let mut field_buf = String::new();

    // The current state of the state machine.
    let mut state = ParseState::FieldStart;

    // We iterate over chars (Unicode scalar values), not bytes. This correctly
    // handles multi-byte UTF-8 characters (e.g., accented letters, emoji).
    // Using .chars() means we never accidentally split a multi-byte sequence.
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        match state {
            // -----------------------------------------------------------------
            // FIELD_START — we are at the beginning of a new field.
            //
            // The very first character tells us which "flavour" of field this is:
            // - '"' → quoted field (everything until the closing '"' is literal)
            // - delimiter → empty unquoted field (zero characters before the delim)
            // - newline → empty unquoted field at end of row
            // - anything else → unquoted field (normal text)
            // -----------------------------------------------------------------
            ParseState::FieldStart => {
                if ch == '"' {
                    // Opening quote: enter quoted mode. The '"' itself is NOT
                    // part of the field value — it is just a syntactic marker.
                    state = ParseState::InQuotedField;
                } else if ch == delimiter {
                    // Delimiter immediately after field start means empty field.
                    // Example: `a,,b` — the middle field is "".
                    current_row.push(String::new());
                    // state stays FieldStart: we are now at the start of the
                    // NEXT field.
                } else if is_newline_start(ch, &chars, i) {
                    // A newline at field start means an empty trailing field on
                    // this row (e.g., the row "a,b," where after the last ","
                    // there is nothing before the newline).
                    //
                    // However, we only emit an empty field here if we have
                    // already seen at least one field on this row (i.e., if
                    // the row is non-empty). This prevents a trailing newline
                    // at the end of the file from producing a spurious empty row.
                    if !current_row.is_empty() {
                        current_row.push(String::new());
                    }
                    // Consume a "\r\n" pair as a single newline.
                    if ch == '\r' && i + 1 < len && chars[i + 1] == '\n' {
                        i += 1; // skip the '\n' too
                    }
                    rows.push(std::mem::take(&mut current_row));
                    // state stays FieldStart for the next row.
                } else {
                    // Regular character: start accumulating an unquoted field.
                    field_buf.push(ch);
                    state = ParseState::InUnquotedField;
                }
            }

            // -----------------------------------------------------------------
            // IN_UNQUOTED_FIELD — accumulating a plain, unquoted field.
            //
            // We append every character to the buffer until we hit a delimiter,
            // newline, or EOF. Quotes in the middle of unquoted fields are
            // treated as literal characters (lenient behaviour — many CSV files
            // have quotes in the middle of strings).
            // -----------------------------------------------------------------
            ParseState::InUnquotedField => {
                if ch == delimiter {
                    // End of this field. Push it and start fresh for the next.
                    current_row.push(std::mem::take(&mut field_buf));
                    state = ParseState::FieldStart;
                } else if is_newline_start(ch, &chars, i) {
                    // End of this field AND end of this row.
                    current_row.push(std::mem::take(&mut field_buf));
                    // Consume "\r\n" pair as one newline.
                    if ch == '\r' && i + 1 < len && chars[i + 1] == '\n' {
                        i += 1;
                    }
                    rows.push(std::mem::take(&mut current_row));
                    state = ParseState::FieldStart;
                } else {
                    // Regular character: append and keep going.
                    field_buf.push(ch);
                }
            }

            // -----------------------------------------------------------------
            // IN_QUOTED_FIELD — inside a "..." quoted field.
            //
            // Inside quotes, the ONLY character with special meaning is `"`.
            // Everything else — including the delimiter and newline — is treated
            // as a literal part of the field value.
            //
            // When we see `"`, we do NOT immediately end the field. Instead we
            // move to InQuotedMaybeEnd and wait for the next character.
            // -----------------------------------------------------------------
            ParseState::InQuotedField => {
                if ch == '"' {
                    // Saw a '"'. Is this the closing quote, or the first '"' of
                    // an escaped '""' pair? We cannot know yet — move to the
                    // "maybe end" state and let the next character decide.
                    state = ParseState::InQuotedMaybeEnd;
                } else {
                    // Any other character (including delimiter and newline) is
                    // literal inside a quoted field. This is the key difference
                    // from unquoted fields.
                    //
                    // Note: we do NOT need to special-case "\r\n" here because
                    // we are appending to the field value — the newline IS part
                    // of the field content per the spec.
                    field_buf.push(ch);
                }
            }

            // -----------------------------------------------------------------
            // IN_QUOTED_MAYBE_END — we just saw '"' inside a quoted field.
            //
            // We have two possibilities:
            //
            // 1. The next char is also '"' → this is an ESCAPED quote (RFC 4180
            //    section 2.7: "If fields are not enclosed with double quotes, then
            //    double quotes may not appear inside the fields").
            //    Action: append a single '"' to the buffer, go back to InQuotedField.
            //
            // 2. The next char is the delimiter, a newline, or EOF → the previous
            //    '"' was the CLOSING quote of this field.
            //    Action: push the field, reset.
            //
            // 3. The next char is something else → technically malformed CSV, but
            //    we adopt lenient behaviour: treat the '"' as a closing quote and
            //    continue processing the next character as the start of the next state.
            //    (This handles files like `"hello"world,` where the quote is closed
            //    early. It is wrong CSV but common in the wild.)
            //
            //    Truth table:
            //    ┌──────────────────┬─────────────────────────────────────────┐
            //    │ Next char        │ Action                                  │
            //    ├──────────────────┼─────────────────────────────────────────┤
            //    │ '"'              │ append '"', → InQuotedField             │
            //    │ delimiter        │ push field, → FieldStart                │
            //    │ '\n' / '\r'      │ push field, new row, → FieldStart       │
            //    │ EOF (handled     │ push field, done                        │
            //    │ after loop)      │                                         │
            //    │ other (lenient)  │ append ch, → InUnquotedField            │
            //    └──────────────────┴─────────────────────────────────────────┘
            // -----------------------------------------------------------------
            ParseState::InQuotedMaybeEnd => {
                if ch == '"' {
                    // Escaped double-quote: "" → "
                    field_buf.push('"');
                    state = ParseState::InQuotedField;
                } else if ch == delimiter {
                    // Closing quote followed by delimiter → end of field.
                    current_row.push(std::mem::take(&mut field_buf));
                    state = ParseState::FieldStart;
                } else if is_newline_start(ch, &chars, i) {
                    // Closing quote followed by newline → end of field AND row.
                    current_row.push(std::mem::take(&mut field_buf));
                    if ch == '\r' && i + 1 < len && chars[i + 1] == '\n' {
                        i += 1;
                    }
                    rows.push(std::mem::take(&mut current_row));
                    state = ParseState::FieldStart;
                } else {
                    // Lenient mode: closing quote followed by unexpected char.
                    // Treat the quote as a close and continue with the char
                    // as part of an unquoted continuation. Not spec-compliant
                    // but maximally tolerant of real-world messy files.
                    field_buf.push(ch);
                    state = ParseState::InUnquotedField;
                }
            }
        }

        i += 1;
    }

    // -------------------------------------------------------------------------
    // End of input: handle the final field and final row.
    //
    // After the loop ends, the state machine may be in any of the four states.
    // We must "flush" whatever is in progress:
    //
    // - FieldStart with current_row empty: trailing newline at end of file,
    //   or completely empty input. Nothing to flush.
    // - FieldStart with current_row non-empty: this shouldn't happen (we always
    //   flush the row when we see a newline) but defensively flush anyway.
    // - InUnquotedField: flush the accumulated buffer as the last field.
    // - InQuotedField: the quote was never closed → UnclosedQuote error.
    // - InQuotedMaybeEnd: the last character of input was '"' which was the
    //   closing quote of the field. This is valid — flush the field.
    // -------------------------------------------------------------------------
    match state {
        ParseState::InQuotedField => {
            // We ended while still inside a quoted field. The opening '"' was
            // never matched by a closing '"'. This is unambiguously an error.
            return Err(CsvError::UnclosedQuote);
        }

        ParseState::InUnquotedField => {
            // The input ended in the middle of an unquoted field (no trailing
            // newline). This is valid — RFC 4180 says the last record MAY omit
            // the trailing CRLF. Push the final field and row.
            current_row.push(std::mem::take(&mut field_buf));
        }

        ParseState::InQuotedMaybeEnd => {
            // The very last character was '"', which we interpret as the closing
            // quote of the current field. The field is complete. Push it.
            current_row.push(std::mem::take(&mut field_buf));
        }

        ParseState::FieldStart => {
            // Nothing in progress. If there is a row in flight (e.g., the input
            // ended with a delimiter but no trailing newline), flush it. But
            // if current_row is empty, we do nothing (empty file or trailing newline).
            //
            // We do NOT push an empty trailing field here because FieldStart at EOF
            // after a newline means the file ended cleanly.
        }
    }

    // If there are fields collected (didn't end with a newline), push the last row.
    if !current_row.is_empty() {
        rows.push(current_row);
    }

    Ok(rows)
}

// ===========================================================================
// Internal: build_row_map
// ===========================================================================

/// Zip a header row and a data row into a `HashMap<String, String>`.
///
/// Handles ragged rows per the spec:
/// - If the data row has **fewer** fields than the header, missing fields are
///   filled with the empty string `""`.
/// - If the data row has **more** fields than the header, extra fields are
///   silently discarded.
///
/// # Example
///
/// ```text
/// header: ["name", "age", "city"]
/// data:   ["Alice", "30"]          ← shorter than header
///
/// result: {"name": "Alice", "age": "30", "city": ""}
///                                                 ^^^^ padded
/// ```
fn build_row_map(header: &[String], data: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for (idx, col_name) in header.iter().enumerate() {
        // Use the data field at this index if it exists; otherwise use "".
        // `get(idx)` returns `None` if the slice is shorter than `idx + 1`.
        let value = data.get(idx).cloned().unwrap_or_default();
        map.insert(col_name.clone(), value);
    }

    map
}

// ===========================================================================
// Internal: is_newline_start
// ===========================================================================

/// Returns `true` if the character at position `i` is the start of a newline
/// sequence (`\n`, `\r`, or `\r\n`).
///
/// We check for `\r` separately from `\n` because Windows line endings are
/// `\r\n`. The caller is responsible for consuming the extra `\n` when it
/// sees `\r` followed by `\n` (to avoid treating `\r\n` as two newlines).
///
/// This function does NOT consume `\r\n` pairs — it just returns `true` for
/// the `\r`. The state machine branches that call this function handle the
/// lookahead and index advance themselves.
#[inline]
fn is_newline_start(ch: char, _chars: &[char], _i: usize) -> bool {
    ch == '\n' || ch == '\r'
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Helper: sort a row map's keys for deterministic assertions.
    // (HashMap ordering is non-deterministic, so we cannot compare maps
    // directly without sorting in some assertion helpers.)
    //
    // We use individual key access (`row["key"]`) rather than full-map equality
    // to keep tests readable.
    // -------------------------------------------------------------------------

    // =========================================================================
    // Basic parsing
    // =========================================================================

    #[test]
    fn test_simple_three_column_table() {
        // The canonical CSV example: names, ages, and cities.
        // This tests that:
        // - The first row is the header (not a data row)
        // - Values are strings (not integers)
        // - Multiple rows are returned
        let csv = "name,age,city\nAlice,30,New York\nBob,25,London\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["age"], "30"); // "30", not 30
        assert_eq!(rows[0]["city"], "New York");
        assert_eq!(rows[1]["name"], "Bob");
        assert_eq!(rows[1]["age"], "25");
        assert_eq!(rows[1]["city"], "London");
    }

    #[test]
    fn test_no_trailing_newline() {
        // RFC 4180 allows the last record to omit the trailing newline.
        // Many tools produce CSV without a trailing newline.
        let csv = "name,value\nhello,world";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "hello");
        assert_eq!(rows[0]["value"], "world");
    }

    #[test]
    fn test_single_column() {
        let csv = "fruit\napple\nbanana\ncherry\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["fruit"], "apple");
        assert_eq!(rows[1]["fruit"], "banana");
        assert_eq!(rows[2]["fruit"], "cherry");
    }

    // =========================================================================
    // Quoted fields
    // =========================================================================

    #[test]
    fn test_quoted_field_with_embedded_comma() {
        // The whole point of quoting: commas inside quotes are not delimiters.
        let csv = "product,price,description\nWidget,9.99,\"A small, round widget\"\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["product"], "Widget");
        assert_eq!(rows[0]["price"], "9.99");
        assert_eq!(rows[0]["description"], "A small, round widget");
    }

    #[test]
    fn test_quoted_field_with_embedded_newline() {
        // Quoted fields can span multiple lines. The newline is part of the value.
        let csv = "id,note\n1,\"Line one\nLine two\"\n2,Single line\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], "1");
        assert_eq!(rows[0]["note"], "Line one\nLine two");
        assert_eq!(rows[1]["id"], "2");
        assert_eq!(rows[1]["note"], "Single line");
    }

    #[test]
    fn test_escaped_double_quote_inside_quoted_field() {
        // "" inside a quoted field represents a single literal ".
        let csv = "id,value\n1,\"She said \"\"hello\"\"\"\n2,plain\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], "1");
        assert_eq!(rows[0]["value"], "She said \"hello\"");
        assert_eq!(rows[1]["value"], "plain");
    }

    #[test]
    fn test_quoted_field_at_start_of_row() {
        let csv = "a,b\n\"quoted start\",normal\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows[0]["a"], "quoted start");
        assert_eq!(rows[0]["b"], "normal");
    }

    #[test]
    fn test_all_fields_quoted() {
        let csv = "\"name\",\"age\"\n\"Alice\",\"30\"\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["age"], "30");
    }

    #[test]
    fn test_empty_quoted_field() {
        // An empty quoted field is just "": two double-quotes with nothing between.
        let csv = "a,b,c\n1,\"\",3\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows[0]["a"], "1");
        assert_eq!(rows[0]["b"], "");
        assert_eq!(rows[0]["c"], "3");
    }

    // =========================================================================
    // Empty fields
    // =========================================================================

    #[test]
    fn test_empty_fields_middle() {
        // a,,b → three fields: "a", "", "b"
        let csv = "a,b,c\n1,,3\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows[0]["a"], "1");
        assert_eq!(rows[0]["b"], "");
        assert_eq!(rows[0]["c"], "3");
    }

    #[test]
    fn test_empty_fields_leading_and_trailing() {
        // ,2, → three fields: "", "2", ""
        let csv = "a,b,c\n,2,\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows[0]["a"], "");
        assert_eq!(rows[0]["b"], "2");
        assert_eq!(rows[0]["c"], "");
    }

    #[test]
    fn test_all_empty_fields() {
        let csv = "a,b,c\n,,\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows[0]["a"], "");
        assert_eq!(rows[0]["b"], "");
        assert_eq!(rows[0]["c"], "");
    }

    // =========================================================================
    // Ragged rows (mismatched column counts)
    // =========================================================================

    #[test]
    fn test_short_row_padded_with_empty_strings() {
        // Row has 2 fields but header has 3. Missing field "city" should be "".
        let csv = "name,age,city\nAlice,30\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["age"], "30");
        assert_eq!(rows[0]["city"], "");
    }

    #[test]
    fn test_long_row_truncated_to_header_length() {
        // Row has 4 fields but header has 3. Extra field should be discarded.
        let csv = "a,b,c\n1,2,3,4\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["a"], "1");
        assert_eq!(rows[0]["b"], "2");
        assert_eq!(rows[0]["c"], "3");
        // Field "4" has no column name and is silently dropped.
        assert_eq!(rows[0].len(), 3);
    }

    // =========================================================================
    // Edge cases: empty / header-only input
    // =========================================================================

    #[test]
    fn test_empty_string_returns_empty_vec() {
        let rows = parse_csv("").unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_header_only_returns_empty_vec() {
        // One row (the header) but no data rows.
        let csv = "name,age,city\n";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_header_only_no_trailing_newline() {
        let csv = "name,age";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 0);
    }

    // =========================================================================
    // Windows line endings (\r\n)
    // =========================================================================

    #[test]
    fn test_crlf_line_endings() {
        // Windows line endings: \r\n should be treated as a single newline.
        let csv = "name,age\r\nAlice,30\r\nBob,25\r\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["name"], "Bob");
    }

    #[test]
    fn test_cr_only_line_endings() {
        // Old Mac line endings: just \r.
        let csv = "name,age\rAlice,30\rBob,25\r";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["name"], "Bob");
    }

    // =========================================================================
    // Custom delimiter
    // =========================================================================

    #[test]
    fn test_tab_delimiter_tsv() {
        let tsv = "name\tage\nAlice\t30\nBob\t25\n";
        let rows = parse_csv_with_delimiter(tsv, '\t').unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["age"], "30");
        assert_eq!(rows[1]["name"], "Bob");
    }

    #[test]
    fn test_semicolon_delimiter() {
        // European CSV style
        let csv = "name;age;city\nAlice;30;Paris\n";
        let rows = parse_csv_with_delimiter(csv, ';').unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["city"], "Paris");
    }

    #[test]
    fn test_pipe_delimiter() {
        let csv = "a|b|c\n1|2|3\n";
        let rows = parse_csv_with_delimiter(csv, '|').unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["a"], "1");
        assert_eq!(rows[0]["b"], "2");
        assert_eq!(rows[0]["c"], "3");
    }

    // =========================================================================
    // Error handling
    // =========================================================================

    #[test]
    fn test_unclosed_quote_returns_error() {
        // An opening '"' that is never closed should return UnclosedQuote.
        let csv = "name,value\n1,\"unclosed\n";
        let result = parse_csv(csv);

        assert_eq!(result, Err(CsvError::UnclosedQuote));
    }

    #[test]
    fn test_unclosed_quote_error_display() {
        let err = CsvError::UnclosedQuote;
        let msg = format!("{}", err);
        assert!(msg.contains("unclosed"));
        assert!(msg.contains("quoted field"));
    }

    // =========================================================================
    // Whitespace preservation
    // =========================================================================

    #[test]
    fn test_whitespace_is_preserved_in_unquoted_fields() {
        // Per spec: whitespace is significant. "  hello  " stays "  hello  ".
        let csv = "key,value\nspaced,  hello  \n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows[0]["value"], "  hello  ");
    }

    #[test]
    fn test_whitespace_preserved_in_quoted_fields() {
        let csv = "key,value\nspaced,\"  hello  \"\n";
        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows[0]["value"], "  hello  ");
    }

    // =========================================================================
    // Multi-row multi-column integration tests
    // =========================================================================

    #[test]
    fn test_realistic_products_table() {
        let csv = "\
product,price,description,in_stock\n\
Widget,9.99,\"A small, round widget\",true\n\
Gadget,19.99,Electronic device,false\n\
Doohickey,4.50,\"Says \"\"hello\"\"\",true\n";

        let rows = parse_csv(csv).unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["product"], "Widget");
        assert_eq!(rows[0]["description"], "A small, round widget");
        assert_eq!(rows[1]["product"], "Gadget");
        assert_eq!(rows[1]["description"], "Electronic device");
        assert_eq!(rows[2]["product"], "Doohickey");
        assert_eq!(rows[2]["description"], "Says \"hello\"");
    }

    #[test]
    fn test_csv_with_numbers_as_strings() {
        // Emphasise that ALL values come back as strings, including numbers.
        let csv = "x,y,z\n1,2,3\n10,20,30\n";
        let rows = parse_csv(csv).unwrap();

        // The value "1" is a &str, not an integer.
        assert_eq!(rows[0]["x"], "1");
        assert_eq!(rows[1]["z"], "30");
        // Type coercion is the caller's job — not ours.
    }

    // =========================================================================
    // CsvError derives
    // =========================================================================

    #[test]
    fn test_csv_error_debug_and_partialeq() {
        let e1 = CsvError::UnclosedQuote;
        let e2 = CsvError::UnclosedQuote;
        assert_eq!(e1, e2);
        let debug_str = format!("{:?}", e1);
        assert!(debug_str.contains("UnclosedQuote"));
    }
}
