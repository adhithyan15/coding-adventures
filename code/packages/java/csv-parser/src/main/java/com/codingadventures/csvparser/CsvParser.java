// ============================================================================
// CsvParser.java — RFC 4180-compatible CSV Parser (Hand-Rolled State Machine)
// ============================================================================
//
// What is CSV?
// ------------
// CSV (Comma-Separated Values) is the world's most common data interchange
// format. Spreadsheets export it, databases dump it, scientists share it.
// Despite its ubiquity, CSV has no single standard. RFC 4180 (2005) is the
// closest thing, but real-world files deviate from it constantly.
//
// This package implements a pragmatic dialect:
//
//   • First row is always the header (defines column names)
//   • All values returned as Strings — no type coercion
//   • Quoted fields can contain commas, newlines, and "" (escaped double-quote)
//   • Configurable delimiter (default: comma)
//   • Ragged rows: short rows padded with "", long rows truncated
//   • Unclosed quoted field → CsvParseException
//
// Why a Hand-Rolled Parser?
// -------------------------
// CSV cannot be tokenized with a simple regex. Consider:
//
//   field1,"field,with,commas",field3
//
// The commas inside the quoted field are not delimiters — but the parser can
// only know that after entering quoted mode. This context-sensitivity means
// CSV parsers are typically hand-rolled character-by-character state machines.
//
// Think of it like reading aloud: when you see a '"', you enter "quoted mode"
// and treat everything differently until the closing '"'.
//
// The State Machine
// -----------------
// The parser uses exactly four states:
//
//                   ┌──────────────┐
//                   │  FIELD_START │◄──────────────────────────────────┐
//                   └──────┬───────┘                                   │
//                          │                                           │
//             ┌────────────┼──────────────────┐                        │
//             │            │                  │                        │
//            '"'        other char      DELIMITER or NEWLINE           │
//             │            │                  │                        │
//             ▼            ▼                  │  emit empty field      │
//      ┌────────────┐  ┌────────────┐          └───────────────────────┘
//      │  IN_QUOTED  │  │IN_UNQUOTED│
//      │   _FIELD    │  │  _FIELD   │
//      └──────┬──────┘  └─────┬─────┘
//             │               │
//       '"'   │          DELIMITER → end field
//             │          NEWLINE   → end row
//             ▼          EOF       → end file
//    ┌──────────────────┐
//    │IN_QUOTED_MAYBE_END│
//    └──────┬────────────┘
//           │
//      ┌────┴────┐
//     '"'    DELIMITER/NEWLINE/EOF
//      │          │
//   escaped    end field
//    quote
//   append '"'
//   back to IN_QUOTED_FIELD
//
// Truth Table: IN_QUOTED_MAYBE_END Transitions
// ─────────────────────────────────────────────────────────────────────
// Previous | Next char    | Interpretation          | Action
// char     |              |                         |
// ─────────┼──────────────┼─────────────────────────┼─────────────────
//    "     | "            | escaped quote ("")      | emit '"', stay quoted
//    "     | DELIMITER    | end of quoted field     | emit field, next field
//    "     | \n or \r     | end of quoted field     | emit field, next row
//    "     | EOF          | end of quoted field     | emit field, end file
//    "     | other        | malformed (tolerant)    | end field, re-process
//
// Spec: code/specs/csv-parser.md
// ============================================================================

package com.codingadventures.csvparser;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * A hand-rolled, RFC 4180-compatible CSV parser.
 *
 * <p>Use {@link #parseCSV(String)} for comma-delimited CSV, or
 * {@link #parseCSVWithDelimiter(String, char)} for a custom delimiter.
 *
 * <h2>Design</h2>
 *
 * <p>All parsing state is encapsulated in the inner {@link Parser} class. The
 * public API is a thin wrapper that constructs a {@link Parser}, calls
 * {@link Parser#run()}, and converts the raw rows into a
 * {@code List<Map<String,String>>} using the first row as the header.
 */
public final class CsvParser {

    // Prevent instantiation — this is a utility class.
    private CsvParser() {}

    // =========================================================================
    // ParseState — the four states of the CSV state machine
    // =========================================================================
    //
    // Using a named enum makes the code self-documenting. Each state has a
    // clear comment describing its role and what transitions leave it.

    /**
     * The current state of the CSV parser's state machine.
     *
     * <p>The parser is always in exactly one of these four states.
     */
    enum ParseState {
        /**
         * Initial state; re-entered after finishing each field. The parser is
         * about to read the first character of a new field.
         *
         * <p>Transitions:
         * <ul>
         *   <li>{@code '"'} → {@code IN_QUOTED_FIELD} (enter quoted mode)</li>
         *   <li>delimiter → {@code FIELD_START} (empty field "")</li>
         *   <li>{@code '\n'}/{@code '\r'} → {@code FIELD_START} (end of row)</li>
         *   <li>EOF → done</li>
         *   <li>other → {@code IN_UNQUOTED_FIELD}</li>
         * </ul>
         */
        FIELD_START,

        /**
         * Collecting a plain (unquoted) field. Stay here until delimiter,
         * newline, or EOF.
         *
         * <p>Transitions:
         * <ul>
         *   <li>delimiter → {@code FIELD_START} (end field, next field)</li>
         *   <li>{@code '\n'}/{@code '\r'} → {@code FIELD_START} (end row)</li>
         *   <li>EOF → done</li>
         *   <li>other → {@code IN_UNQUOTED_FIELD} (keep collecting)</li>
         * </ul>
         */
        IN_UNQUOTED_FIELD,

        /**
         * Collecting a quoted field. Inside a quoted field, almost everything
         * is literal — commas, newlines, backslashes. Only {@code '"'} is special.
         *
         * <p>Transitions:
         * <ul>
         *   <li>{@code '"'} → {@code IN_QUOTED_MAYBE_END}</li>
         *   <li>EOF → error (unclosed quote)</li>
         *   <li>other → {@code IN_QUOTED_FIELD} (literal character)</li>
         * </ul>
         */
        IN_QUOTED_FIELD,

        /**
         * State after seeing {@code '"'} inside a quoted field. The next
         * character determines whether this was an escaped {@code ""} or the
         * end of the quoted field.
         *
         * <p>Transitions:
         * <ul>
         *   <li>{@code '"'} → {@code IN_QUOTED_FIELD} (escape: emit {@code '"'})</li>
         *   <li>delimiter → {@code FIELD_START} (end field)</li>
         *   <li>{@code '\n'}/{@code '\r'} → {@code FIELD_START} (end row)</li>
         *   <li>EOF → done (field ended cleanly)</li>
         *   <li>other → {@code FIELD_START} (tolerant: end field, re-process)</li>
         * </ul>
         */
        IN_QUOTED_MAYBE_END
    }

    // =========================================================================
    // CsvParseException
    // =========================================================================

    /**
     * Thrown when the parser encounters malformed CSV input.
     *
     * <p>Currently this only occurs for unclosed quoted fields.
     */
    public static class CsvParseException extends Exception {
        /**
         * Constructs a {@link CsvParseException} with the given message.
         *
         * @param message describes what went wrong
         */
        public CsvParseException(String message) {
            super("csv parse error: " + message);
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Parses CSV text using the default comma delimiter.
     *
     * <p>Returns a list of maps from column name to string value. The first row
     * of the input is treated as the header and defines the map keys; it does
     * not appear in the returned list.
     *
     * <p>Edge cases:
     * <ul>
     *   <li>Empty input → empty list, no error</li>
     *   <li>Header-only input → empty list, no error</li>
     *   <li>Ragged rows → short rows padded with {@code ""}, long rows
     *       truncated</li>
     * </ul>
     *
     * <pre>
     *   List&lt;Map&lt;String,String&gt;&gt; rows = CsvParser.parseCSV("name,age\nAlice,30\nBob,25\n");
     *   // rows.get(0).get("name") == "Alice"
     *   // rows.get(0).get("age")  == "30"
     * </pre>
     *
     * @param source the CSV text
     * @return a list of maps, one per data row
     * @throws CsvParseException if the input contains an unclosed quoted field
     */
    public static List<Map<String, String>> parseCSV(String source)
            throws CsvParseException {
        return parseCSVWithDelimiter(source, ',');
    }

    /**
     * Like {@link #parseCSV(String)} but with a configurable delimiter.
     *
     * <p>The delimiter parameter is a single character. Common choices:
     * <ul>
     *   <li>{@code ','} — comma (the default)</li>
     *   <li>{@code '\t'} — tab (TSV)</li>
     *   <li>{@code ';'} — semicolon (common in European locales)</li>
     *   <li>{@code '|'} — pipe-separated</li>
     * </ul>
     *
     * @param source    the CSV text
     * @param delimiter the field separator character
     * @return a list of maps, one per data row
     * @throws CsvParseException if the input contains an unclosed quoted field
     */
    public static List<Map<String, String>> parseCSVWithDelimiter(
            String source, char delimiter) throws CsvParseException {
        Parser p = new Parser(source, delimiter);
        p.run();
        return buildRowMaps(p.header(), p.dataRows());
    }

    // =========================================================================
    // Parser — internal state machine
    // =========================================================================
    //
    // We encapsulate all mutable parser state in a private static inner class
    // rather than passing it through function parameters. This is idiomatic for
    // a hand-rolled Java parser: methods on Parser update the state in place,
    // keeping the public API clean.

    /**
     * Internal mutable state for an in-progress CSV parse.
     */
    private static final class Parser {

        // The input as a char array. We use char[] rather than String to avoid
        // repeated charAt() bounds checks and to allow direct array indexing.
        // Java's char is 16-bit UTF-16, which handles all BMP characters.
        private final char[] chars;

        // Current read position within chars.
        private int pos;

        // Current state machine state.
        private ParseState state;

        // Accumulates characters for the current field.
        // StringBuilder grows dynamically — O(n) total allocation per parse.
        private final StringBuilder fieldBuf;

        // Accumulates field values for the current row.
        private final List<String> currentRow;

        // Accumulates completed rows. Each row is a List<String> of field values.
        private final List<List<String>> rows;

        // The field delimiter (e.g., ',', '\t', ';').
        private final char delim;

        Parser(String source, char delimiter) {
            this.chars = source.toCharArray();
            this.pos = 0;
            this.state = ParseState.FIELD_START;
            this.fieldBuf = new StringBuilder();
            this.currentRow = new ArrayList<>();
            this.rows = new ArrayList<>();
            this.delim = delimiter;
        }

        // =====================================================================
        // run — drives the state machine
        // =====================================================================
        //
        // run processes the entire input by stepping the state machine one
        // character at a time. At EOF it flushes any in-progress field and row.
        //
        // The loop goes to pos == chars.length (inclusive) so that we process
        // the EOF "virtual character" once. We signal EOF with atEOF=true.

        void run() throws CsvParseException {
            while (pos <= chars.length) {
                boolean atEOF = (pos == chars.length);
                char c = atEOF ? 0 : chars[pos];

                step(c, atEOF);

                if (atEOF) break;
                pos++;
            }
        }

        // =====================================================================
        // step — process one character (or EOF)
        // =====================================================================
        //
        // The logic is a switch on (state, char) that mirrors the state machine
        // diagram in the package comment. When atEOF is true, the value of c is
        // irrelevant — we signal that input has ended.

        void step(char c, boolean atEOF) throws CsvParseException {
            switch (state) {

                // ── FIELD_START ──────────────────────────────────────────────
                case FIELD_START: {
                    if (atEOF) {
                        // EOF while at field_start: flush if we have an in-
                        // progress row (trailing newline → don't emit empty row).
                        if (!currentRow.isEmpty()) {
                            flushRow();
                        }
                        return;
                    }

                    if (c == '"') {
                        // Opening double-quote: enter quoted mode.
                        // The '"' is structural — don't add it to the buffer.
                        state = ParseState.IN_QUOTED_FIELD;
                    } else if (c == delim) {
                        // Delimiter at field start: current field is empty ("").
                        finishField();
                        // state stays at FIELD_START for the next field
                    } else if (c == '\r') {
                        // '\r' at field start: possible \r\n pair or bare \r.
                        handleCR();
                    } else if (c == '\n') {
                        // '\n' with a non-empty row: last field before newline
                        // was empty (delimiter immediately before newline).
                        // With an empty row: blank line — skip it.
                        if (!currentRow.isEmpty()) {
                            currentRow.add("");
                            flushRow();
                        }
                    } else {
                        // Any other character: start of an unquoted field.
                        fieldBuf.append(c);
                        state = ParseState.IN_UNQUOTED_FIELD;
                    }
                    return;
                }

                // ── IN_UNQUOTED_FIELD ────────────────────────────────────────
                case IN_UNQUOTED_FIELD: {
                    if (atEOF) {
                        // EOF while collecting unquoted field: flush both.
                        finishField();
                        flushRow();
                        return;
                    }

                    if (c == delim) {
                        finishField();
                        state = ParseState.FIELD_START;
                    } else if (c == '\r') {
                        finishField();
                        flushRow();
                        handleCR();
                        state = ParseState.FIELD_START;
                    } else if (c == '\n') {
                        finishField();
                        flushRow();
                        state = ParseState.FIELD_START;
                    } else {
                        fieldBuf.append(c);
                    }
                    return;
                }

                // ── IN_QUOTED_FIELD ──────────────────────────────────────────
                case IN_QUOTED_FIELD: {
                    if (atEOF) {
                        // EOF inside a quoted field: malformed input.
                        throw new CsvParseException(
                            "unclosed quoted field at end of input");
                    }

                    if (c == '"') {
                        // Could be end-of-field or start of "" escape.
                        state = ParseState.IN_QUOTED_MAYBE_END;
                    } else {
                        // Literal character: commas, newlines, etc. are allowed
                        // inside a quoted field.
                        fieldBuf.append(c);
                    }
                    return;
                }

                // ── IN_QUOTED_MAYBE_END ──────────────────────────────────────
                case IN_QUOTED_MAYBE_END: {
                    if (atEOF) {
                        // EOF after closing '"': field ended cleanly.
                        finishField();
                        flushRow();
                        return;
                    }

                    if (c == '"') {
                        // Another '"': this is a "" escape.
                        // Append one literal '"' and return to IN_QUOTED_FIELD.
                        //
                        // Walking through `"say ""hello"""`  →  say "hello"
                        //   After opening '"':      state=IN_QUOTED
                        //   After 's','a','y',' ':   buf="say "
                        //   First '"' of "":        state=IN_QUOTED_MAYBE_END
                        //   Second '"' of "":       state=IN_QUOTED, buf="say \""
                        //   After 'h','e','l','l','o': buf="say \"hello"
                        //   First '"' of "":        state=IN_QUOTED_MAYBE_END
                        //   Second '"' of "":       state=IN_QUOTED, buf="say \"hello\""
                        //   Final '"':              state=IN_QUOTED_MAYBE_END
                        //   EOF/delim:              emit field  say "hello"
                        fieldBuf.append('"');
                        state = ParseState.IN_QUOTED_FIELD;
                    } else if (c == delim) {
                        // Delimiter after closing '"': end of quoted field.
                        finishField();
                        state = ParseState.FIELD_START;
                    } else if (c == '\r') {
                        finishField();
                        flushRow();
                        handleCR();
                        state = ParseState.FIELD_START;
                    } else if (c == '\n') {
                        finishField();
                        flushRow();
                        state = ParseState.FIELD_START;
                    } else {
                        // Any other character: malformed per RFC 4180.
                        // We tolerate it: end the quoted field and re-process
                        // this character from FIELD_START on the next step.
                        //
                        // To re-process: finish field, set state to FIELD_START,
                        // then decrement pos so the main loop re-increments it
                        // and feeds this character again.
                        finishField();
                        state = ParseState.FIELD_START;
                        pos--; // re-process this character
                    }
                    return;
                }
            }
        }

        // =====================================================================
        // Helper methods
        // =====================================================================

        /**
         * Completes the current field: appends the buffer contents to the
         * current row and resets the buffer.
         *
         * <p>An empty buffer produces the empty string {@code ""}.
         */
        private void finishField() {
            currentRow.add(fieldBuf.toString());
            fieldBuf.setLength(0); // reset without allocating a new StringBuilder
        }

        /**
         * Completes the current row: appends currentRow to rows and resets
         * currentRow to an empty list.
         */
        private void flushRow() {
            rows.add(new ArrayList<>(currentRow)); // defensive copy
            currentRow.clear();
        }

        /**
         * Handles a carriage return character ({@code '\r'}).
         *
         * <p>If the next character is {@code '\n'} (forming a Windows
         * {@code \r\n} line ending), we advance past it. This way the main
         * loop never sees the {@code '\n'} of a {@code \r\n} pair and won't
         * double-process the end-of-row.
         */
        private void handleCR() {
            int next = pos + 1;
            if (next < chars.length && chars[next] == '\n') {
                pos++; // skip the '\n' of a \r\n pair
            }
        }

        /**
         * Returns the first row (the header). Returns {@code null} if there
         * are no rows at all.
         */
        List<String> header() {
            return rows.isEmpty() ? null : rows.get(0);
        }

        /**
         * Returns all rows after the first (the data rows). Returns an empty
         * list if there are fewer than 2 rows.
         */
        List<List<String>> dataRows() {
            if (rows.size() <= 1) return Collections.emptyList();
            return rows.subList(1, rows.size());
        }
    }

    // =========================================================================
    // buildRowMaps — convert row slices to maps using the header as keys
    // =========================================================================
    //
    // buildRowMaps zips each data row with the header to produce a list of maps.
    //
    // Handles ragged rows:
    //
    //   Short row: pad missing fields with ""
    //
    //     header: ["a", "b", "c"]
    //     row:    ["1", "2"]         ← missing "c"
    //     result: {"a":"1","b":"2","c":""}
    //
    //   Long row: truncate extra fields
    //
    //     header: ["a", "b"]
    //     row:    ["1", "2", "3"]    ← extra "3" discarded
    //     result: {"a":"1","b":"2"}
    //
    // We use LinkedHashMap to preserve header insertion order in the output
    // maps, so iteration order matches the column order from the CSV file.

    private static List<Map<String, String>> buildRowMaps(
            List<String> header, List<List<String>> dataRows) {
        if (header == null || dataRows.isEmpty()) {
            return Collections.emptyList();
        }

        List<Map<String, String>> result = new ArrayList<>(dataRows.size());

        for (List<String> row : dataRows) {
            // LinkedHashMap preserves column insertion order.
            Map<String, String> m = new LinkedHashMap<>(header.size());

            for (int i = 0; i < header.size(); i++) {
                if (i < row.size()) {
                    // Field exists in this row: use it.
                    m.put(header.get(i), row.get(i));
                } else {
                    // Field is missing (short row): pad with "".
                    m.put(header.get(i), "");
                }
            }
            // Fields beyond header.size() are silently dropped (long rows).

            result.add(m);
        }

        return result;
    }
}
