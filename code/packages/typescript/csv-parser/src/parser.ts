/**
 * parser.ts — hand-rolled state machine CSV parser.
 *
 * # Why a state machine?
 *
 * CSV is context-sensitive: the meaning of a comma depends on whether you are
 * currently inside a quoted field. You cannot lex CSV with a regular tokenizer
 * because you must track state as you scan.
 *
 * A state machine is the natural fit. At any point in the input, the parser is
 * in exactly one state. Each character causes either a state transition, an
 * action (append character, emit field, start new row), or both.
 *
 * # The four states
 *
 * ```
 * ┌────────────────────────────────────────────────────────────────────────┐
 * │                                                                        │
 * │  FIELD_START                                                           │
 * │      │                                                                 │
 * │      ├─── '"' ──────────────────────────► IN_QUOTED_FIELD             │
 * │      │                                         │                      │
 * │      │                          other char ────┤ append to buffer     │
 * │      │                                         │                      │
 * │      │                          '"' ───────────► IN_QUOTED_MAYBE_END  │
 * │      │                                              │                 │
 * │      │                          '"' again ──────────┤                 │
 * │      │                          (append '"', back to IN_QUOTED)      │
 * │      │                                              │                 │
 * │      │                          delimiter/newline/EOF → end field     │
 * │      │                                                                │
 * │      └─── other ────────────────────────► IN_UNQUOTED_FIELD          │
 * │                                                  │                   │
 * │                                     other char ──┤ append to buffer  │
 * │                                                  │                   │
 * │                                     delimiter ───► end field, next   │
 * │                                     newline ──────► end field, row   │
 * │                                     EOF ───────────► end field, done │
 * │                                                                       │
 * └───────────────────────────────────────────────────────────────────────┘
 * ```
 *
 * # Grammar
 *
 * ```
 * file         = [header] { record }
 * header       = record              -- first row is the header
 * record       = field { DELIM field } (NEWLINE | EOF)
 * field        = quoted | unquoted
 * quoted       = '"' { QCHAR | DELIM | NEWLINE | '""' } '"'
 * unquoted     = { UCHAR }           -- may be empty string ""
 *
 * DELIM        = ","  (configurable)
 * NEWLINE      = "\r\n" | "\n" | "\r"
 * QCHAR        = any char except '"'
 * UCHAR        = any char except DELIM, '"', NEWLINE, EOF
 * ```
 */

import { type CsvRow, type ParseState } from "./types.js";
import { UnclosedQuoteError } from "./errors.js";

// ============================================================================
// Public API
// ============================================================================

/**
 * Parse CSV text using the default comma delimiter.
 *
 * The first row is treated as the header. All values are returned as strings.
 *
 * @param source - The full CSV text as a string. May be empty.
 * @returns An array of row objects. Each object maps column name → field value.
 * @throws {UnclosedQuoteError} if a quoted field is not closed before EOF.
 *
 * @example
 * ```typescript
 * const csv = "name,age,city\nAlice,30,New York\nBob,25,London\n";
 * const rows = parseCSV(csv);
 * // rows[0] === { name: "Alice", age: "30", city: "New York" }
 * // rows[1] === { name: "Bob",   age: "25", city: "London"   }
 * ```
 */
export function parseCSV(source: string): CsvRow[] {
  return parseCSVWithDelimiter(source, ",");
}

/**
 * Parse CSV text with a configurable field delimiter.
 *
 * Identical to `parseCSV`, but accepts any single character as the field
 * delimiter instead of the default comma.
 *
 * Common alternatives:
 * - `'\t'` — Tab-Separated Values (TSV)
 * - `';'` — European CSV (where `,` is the decimal separator)
 * - `'|'` — Pipe-separated (used in some database exports)
 *
 * @param source - The full CSV text as a string.
 * @param delimiter - A single-character field separator.
 * @returns An array of row objects.
 * @throws {UnclosedQuoteError} if a quoted field is not closed before EOF.
 *
 * @example
 * ```typescript
 * const tsv = "name\tage\nAlice\t30\n";
 * const rows = parseCSVWithDelimiter(tsv, "\t");
 * // rows[0] === { name: "Alice", age: "30" }
 * ```
 */
export function parseCSVWithDelimiter(
  source: string,
  delimiter: string
): CsvRow[] {
  // -------------------------------------------------------------------------
  // Phase 1: tokenise the source into raw rows.
  //
  // We scan character-by-character using the state machine. The result is a
  // string[][] — a list of rows, each row being a list of field strings.
  //
  // We do NOT apply header semantics here. Separating tokenisation from
  // header application makes each phase simpler and individually testable.
  // -------------------------------------------------------------------------
  const rawRows = tokeniseRows(source, delimiter);

  // -------------------------------------------------------------------------
  // Phase 2: apply the header to build the final row objects.
  //
  // The first raw row is the header. Every subsequent raw row is a data row.
  // We zip each data row's fields with the header's column names.
  // -------------------------------------------------------------------------
  if (rawRows.length === 0) {
    // Empty file: no header, no data.
    return [];
  }

  const header = rawRows[0];
  const dataRows = rawRows.slice(1);

  if (dataRows.length === 0) {
    // Header-only file: spec says return empty list.
    return [];
  }

  return dataRows.map((row) => buildRowMap(header, row));
}

// ============================================================================
// Internal: tokeniseRows
// ============================================================================

/**
 * Walk the CSV source character-by-character using the state machine and
 * return a raw `string[][]` — rows of field strings, with no header semantics.
 *
 * This function contains all of the state machine logic. It knows nothing
 * about column names; it just splits the input into rows and fields.
 *
 * # State transition table
 *
 * | Current State        | Character         | Action                         |
 * |----------------------|-------------------|--------------------------------|
 * | FIELD_START          | `"`               | → IN_QUOTED_FIELD              |
 * | FIELD_START          | delimiter         | push "" field, stay FIELD_START|
 * | FIELD_START          | `\n` / `\r`       | end row (if non-empty)         |
 * | FIELD_START          | EOF               | end (nothing to flush)         |
 * | FIELD_START          | other             | append, → IN_UNQUOTED_FIELD    |
 * | IN_UNQUOTED_FIELD    | delimiter         | push field, → FIELD_START      |
 * | IN_UNQUOTED_FIELD    | `\n` / `\r`       | push field, new row            |
 * | IN_UNQUOTED_FIELD    | EOF               | push field, done               |
 * | IN_UNQUOTED_FIELD    | other             | append to buffer               |
 * | IN_QUOTED_FIELD      | `"`               | → IN_QUOTED_MAYBE_END          |
 * | IN_QUOTED_FIELD      | other             | append to buffer               |
 * | IN_QUOTED_MAYBE_END  | `"`               | append `"`, ← IN_QUOTED_FIELD  |
 * | IN_QUOTED_MAYBE_END  | delimiter         | push field, → FIELD_START      |
 * | IN_QUOTED_MAYBE_END  | `\n` / `\r`       | push field, new row            |
 * | IN_QUOTED_MAYBE_END  | EOF               | push field, done               |
 * | IN_QUOTED_MAYBE_END  | other (lenient)   | append ch, → IN_UNQUOTED_FIELD |
 *
 * @param source - Raw CSV text.
 * @param delimiter - Single-character field separator.
 * @returns Array of rows, each row is an array of field strings.
 * @throws {UnclosedQuoteError} if EOF is reached inside a quoted field.
 */
function tokeniseRows(source: string, delimiter: string): string[][] {
  const rows: string[][] = [];
  let currentRow: string[] = [];
  let fieldBuf = "";
  let state: ParseState = "FIELD_START";

  // We iterate by index over the string. Using an explicit index lets us
  // look ahead by one character to handle the "\r\n" two-character newline
  // sequence (consuming both characters as a single logical newline).
  const len = source.length;
  let i = 0;

  while (i < len) {
    const ch = source[i];

    switch (state) {
      // -----------------------------------------------------------------------
      // FIELD_START — beginning of a new field.
      //
      // The first character tells us the field's "flavour":
      // - '"' → quoted field (everything until closing '"' is literal)
      // - delimiter → empty unquoted field
      // - newline → end of row
      // - other → unquoted field
      // -----------------------------------------------------------------------
      case "FIELD_START": {
        if (ch === '"') {
          // Opening quote: enter quoted mode. The '"' itself is NOT part of
          // the field value — it is just a syntactic marker.
          state = "IN_QUOTED_FIELD";
        } else if (ch === delimiter) {
          // Delimiter immediately → empty field.
          // Example: `a,,b` — the middle field is "".
          currentRow.push("");
          // state stays FIELD_START — we are at the start of the NEXT field.
        } else if (isNewlineStart(ch)) {
          // A newline at field start can mean:
          // (a) A truly empty row (blank line): skip it.
          // (b) A trailing empty field on a row that ended with the delimiter.
          //
          // We only push an empty trailing field if we already have fields on
          // this row. This prevents a trailing newline at end-of-file from
          // producing a spurious empty row.
          if (currentRow.length > 0) {
            currentRow.push("");
          }
          i = consumeNewline(source, i); // skip \r\n pair if present
          rows.push(currentRow);
          currentRow = [];
          // state stays FIELD_START for the next row.
        } else {
          // Regular character: start an unquoted field.
          fieldBuf += ch;
          state = "IN_UNQUOTED_FIELD";
        }
        break;
      }

      // -----------------------------------------------------------------------
      // IN_UNQUOTED_FIELD — accumulating a plain, unquoted field.
      //
      // We append every character to the buffer until we hit a delimiter,
      // newline, or EOF. Whitespace is significant — spaces around fields
      // are part of the field value (per spec).
      // -----------------------------------------------------------------------
      case "IN_UNQUOTED_FIELD": {
        if (ch === delimiter) {
          // End of this field. Push buffer, reset.
          currentRow.push(fieldBuf);
          fieldBuf = "";
          state = "FIELD_START";
        } else if (isNewlineStart(ch)) {
          // End of field AND end of row.
          currentRow.push(fieldBuf);
          fieldBuf = "";
          i = consumeNewline(source, i); // skip \r\n pair
          rows.push(currentRow);
          currentRow = [];
          state = "FIELD_START";
        } else {
          // Regular character: append and keep going.
          fieldBuf += ch;
        }
        break;
      }

      // -----------------------------------------------------------------------
      // IN_QUOTED_FIELD — inside a "..." quoted field.
      //
      // Inside quotes, the ONLY character with special meaning is `"`. Everything
      // else — including the delimiter and newlines — is a literal part of the
      // field value.
      //
      // When we see `"`, we cannot immediately close the field because it might
      // be the first `"` of an escape sequence `""`. We move to MAYBE_END and
      // wait for the next character.
      // -----------------------------------------------------------------------
      case "IN_QUOTED_FIELD": {
        if (ch === '"') {
          // Possibly the closing quote, possibly the start of `""` escape.
          // We cannot know yet — delegate to IN_QUOTED_MAYBE_END.
          state = "IN_QUOTED_MAYBE_END";
        } else {
          // Any other character (including delimiter and newline) is literal.
          // Note: we do NOT need to special-case "\r\n" here because we are
          // appending to the field value — the embedded newline IS part of the
          // field per the spec.
          fieldBuf += ch;
        }
        break;
      }

      // -----------------------------------------------------------------------
      // IN_QUOTED_MAYBE_END — we just saw '"' inside a quoted field.
      //
      // We have three cases:
      //
      // 1. Next char is `"` → escaped quote: `""` → append single `"`, back to
      //    IN_QUOTED_FIELD.
      //
      // 2. Next char is delimiter, newline, or EOF → the previous `"` was the
      //    closing quote. Push the field, reset.
      //
      // 3. Next char is something else → lenient mode: treat the `"` as a
      //    closing quote and continue with the unexpected char in unquoted mode.
      //
      // Truth table:
      // ┌────────────────────┬─────────────────────────────────────────────┐
      // │ Next char          │ Action                                      │
      // ├────────────────────┼─────────────────────────────────────────────┤
      // │ '"'                │ append '"', → IN_QUOTED_FIELD               │
      // │ delimiter          │ push field, → FIELD_START                   │
      // │ '\n' / '\r'        │ push field, new row, → FIELD_START          │
      // │ other (lenient)    │ append ch, → IN_UNQUOTED_FIELD              │
      // └────────────────────┴─────────────────────────────────────────────┘
      // -----------------------------------------------------------------------
      case "IN_QUOTED_MAYBE_END": {
        if (ch === '"') {
          // Escaped double-quote: "" → "
          fieldBuf += '"';
          state = "IN_QUOTED_FIELD";
        } else if (ch === delimiter) {
          // Closing quote followed by delimiter → end of field.
          currentRow.push(fieldBuf);
          fieldBuf = "";
          state = "FIELD_START";
        } else if (isNewlineStart(ch)) {
          // Closing quote followed by newline → end of field AND row.
          currentRow.push(fieldBuf);
          fieldBuf = "";
          i = consumeNewline(source, i);
          rows.push(currentRow);
          currentRow = [];
          state = "FIELD_START";
        } else {
          // Lenient mode: closing quote followed by unexpected char.
          // Treat the quote as a close and continue with the char in
          // unquoted mode. Not spec-compliant but tolerant of real-world files.
          fieldBuf += ch;
          state = "IN_UNQUOTED_FIELD";
        }
        break;
      }
    }

    i += 1;
  }

  // --------------------------------------------------------------------------
  // End of input: flush whatever is in progress.
  //
  // After the loop, we may have an incomplete field or row. Each state has
  // different semantics at EOF:
  //
  // - FIELD_START + empty currentRow: clean end (file ended with newline, or empty).
  // - FIELD_START + non-empty currentRow: shouldn't normally happen, but flush.
  // - IN_UNQUOTED_FIELD: flush buffer as the last field (no trailing newline — valid).
  // - IN_QUOTED_FIELD: unclosed quote → error.
  // - IN_QUOTED_MAYBE_END: last char was '"' → it's the closing quote → flush field.
  // --------------------------------------------------------------------------
  if (state === "IN_QUOTED_FIELD") {
    // The opening '"' was never matched. Unambiguous error.
    throw new UnclosedQuoteError();
  }

  if (state === "IN_UNQUOTED_FIELD") {
    // Input ended in the middle of an unquoted field (no trailing newline).
    // RFC 4180 allows this. Push the final field.
    currentRow.push(fieldBuf);
  } else if (state === "IN_QUOTED_MAYBE_END") {
    // The very last character was '"', which was the closing quote of the field.
    // The field is complete.
    currentRow.push(fieldBuf);
  }
  // For FIELD_START: nothing in the buffer, nothing to push (or currentRow
  // was already flushed by the last newline in the loop).

  if (currentRow.length > 0) {
    rows.push(currentRow);
  }

  return rows;
}

// ============================================================================
// Internal: buildRowMap
// ============================================================================

/**
 * Zip a header array and a data row array into a `CsvRow` object.
 *
 * Handles ragged rows per the spec:
 * - If `data` is **shorter** than `header`, missing fields are filled with `""`.
 * - If `data` is **longer** than `header`, extra fields are silently discarded.
 *
 * @example
 * ```
 * header: ["name", "age", "city"]
 * data:   ["Alice", "30"]          ← shorter than header
 *
 * result: { name: "Alice", age: "30", city: "" }
 *                                          ^^^ padded
 * ```
 */
function buildRowMap(header: string[], data: string[]): CsvRow {
  const row: CsvRow = {};

  for (let idx = 0; idx < header.length; idx++) {
    const colName = header[idx];
    // Use the data field at this index if it exists; otherwise use "".
    // The `??` operator returns the right side only if the left side is
    // null or undefined — which is what `data[idx]` returns for out-of-bounds.
    row[colName] = data[idx] ?? "";
  }

  return row;
}

// ============================================================================
// Internal: newline helpers
// ============================================================================

/**
 * Returns `true` if `ch` is the start of a newline sequence.
 *
 * We check for both `\n` and `\r` because CSV files use three different
 * newline conventions in the wild:
 * - `\n` (LF)   — Unix / Linux / macOS (modern)
 * - `\r\n` (CRLF) — Windows / DOS / RFC 4180 native
 * - `\r` (CR)   — Classic Mac OS (very old, but still encountered)
 *
 * The `\r` case is the tricky one: `\r\n` must be treated as a single newline,
 * not two. When `isNewlineStart` returns true for `\r`, the caller should call
 * `consumeNewline` to advance the index past the optional following `\n`.
 */
function isNewlineStart(ch: string): boolean {
  return ch === "\n" || ch === "\r";
}

/**
 * Consume a newline at position `i`, advancing past `\r\n` as a unit.
 *
 * When the character at `i` is `\r` and the next character is `\n`, we must
 * skip both to avoid treating Windows line endings as two separate newlines.
 *
 * Returns the updated index (pointing to the last consumed character, so the
 * caller's `i += 1` at end-of-loop advances to the character after the newline).
 *
 * Examples:
 * - `\n`   at position 5 → returns 5  (caller's i++ → 6, which is next char)
 * - `\r`   at position 5 → returns 5  (single CR; caller's i++ → 6)
 * - `\r\n` at position 5 → returns 6  (caller's i++ → 7, skipping the \n)
 */
function consumeNewline(source: string, i: number): number {
  if (source[i] === "\r" && i + 1 < source.length && source[i + 1] === "\n") {
    // Windows CRLF: consume the \r here, let the loop's `i++` consume the \n.
    return i + 1;
  }
  // Unix \n or old Mac \r: nothing extra to consume.
  return i;
}
