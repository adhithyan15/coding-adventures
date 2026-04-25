// ============================================================================
// CsvParser.kt — RFC 4180-compatible CSV Parser (Hand-Rolled State Machine)
// ============================================================================
//
// What is CSV?
// ------------
// CSV (Comma-Separated Values) is the world's most common data interchange
// format. Spreadsheets export it, databases dump it, scientists share it.
// Despite its ubiquity, CSV has no single standard. RFC 4180 (2005) is the
// closest thing, but real-world files deviate from it constantly.
//
// This implementation follows a pragmatic dialect:
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
//    └──────────────────┘
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

package com.codingadventures.csvparser

/**
 * The current state of the CSV parser's state machine.
 *
 * The parser is always in exactly one of these four states.
 */
enum class ParseState {
    /**
     * Initial state; re-entered after finishing each field. The parser is
     * about to read the first character of a new field.
     */
    FIELD_START,

    /**
     * Collecting a plain (unquoted) field. Stay here until delimiter,
     * newline, or EOF.
     */
    IN_UNQUOTED_FIELD,

    /**
     * Collecting a quoted field. Inside a quoted field, almost everything is
     * literal — commas, newlines, etc. Only `"` is special.
     */
    IN_QUOTED_FIELD,

    /**
     * After seeing `"` inside a quoted field. The next character determines
     * whether this was an escaped `""` or the end of the quoted field.
     */
    IN_QUOTED_MAYBE_END
}

/**
 * Thrown when the parser encounters malformed CSV input.
 *
 * Currently only raised for unclosed quoted fields.
 */
class CsvParseException(message: String) : Exception("csv parse error: $message")

// ============================================================================
// Public API
// ============================================================================

/**
 * Parses CSV text using the default comma delimiter.
 *
 * Returns a list of maps from column name to string value. The first row is
 * treated as the header and defines the map keys; it does not appear in the
 * returned list.
 *
 * Edge cases:
 * - Empty input → empty list, no error
 * - Header-only input → empty list, no error
 * - Ragged rows → short rows padded with `""`, long rows truncated
 *
 * ```kotlin
 * val rows = parseCSV("name,age\nAlice,30\nBob,25\n")
 * rows[0]["name"]  // "Alice"
 * rows[0]["age"]   // "30"
 * ```
 *
 * @param source the CSV text
 * @return a list of maps, one per data row
 * @throws CsvParseException if the input contains an unclosed quoted field
 */
fun parseCSV(source: String): List<Map<String, String>> =
    parseCSVWithDelimiter(source, ',')

/**
 * Like [parseCSV] but with a configurable delimiter.
 *
 * Common choices:
 * - `','` — comma (the default)
 * - `'\t'` — tab (TSV / tab-separated values)
 * - `';'` — semicolon (common in European locales)
 * - `'|'` — pipe-separated
 *
 * @param source    the CSV text
 * @param delimiter the field separator character
 * @return a list of maps, one per data row
 * @throws CsvParseException if the input contains an unclosed quoted field
 */
fun parseCSVWithDelimiter(
    source: String,
    delimiter: Char
): List<Map<String, String>> {
    val p = Parser(source, delimiter)
    p.run()
    return buildRowMaps(p.header(), p.dataRows())
}

// ============================================================================
// Parser — internal state machine
// ============================================================================
//
// All mutable parser state lives in this class rather than being passed through
// function parameters. This is idiomatic for a hand-rolled Kotlin parser:
// methods on Parser update state in place, keeping the public API clean.

/**
 * Internal mutable state for an in-progress CSV parse.
 */
private class Parser(source: String, private val delim: Char) {

    // The input as a CharArray. We iterate by index to allow the
    // IN_QUOTED_MAYBE_END state to "unget" a character by decrementing pos.
    private val chars: CharArray = source.toCharArray()

    // Current read position within chars.
    private var pos: Int = 0

    // Current state machine state.
    private var state: ParseState = ParseState.FIELD_START

    // Accumulates characters for the current field.
    // StringBuilder grows dynamically — O(n) total allocation per parse.
    private val fieldBuf: StringBuilder = StringBuilder()

    // Accumulates field values for the current row.
    private val currentRow: MutableList<String> = mutableListOf()

    // Accumulates completed rows. Each row is a List<String> of field values.
    private val rows: MutableList<List<String>> = mutableListOf()

    // ==========================================================================
    // run — drives the state machine
    // ==========================================================================
    //
    // Processes the entire input by stepping the state machine one character at
    // a time. The loop goes to pos == chars.size (inclusive) so that we process
    // the EOF "virtual character" once (signalled by atEOF=true).

    fun run() {
        while (pos <= chars.size) {
            val atEOF = pos == chars.size
            val c = if (atEOF) '\u0000' else chars[pos]

            step(c, atEOF)

            if (atEOF) break
            pos++
        }
    }

    // ==========================================================================
    // step — process one character (or EOF)
    // ==========================================================================
    //
    // The logic is a when on (state, char) that mirrors the state machine
    // diagram in the file header. When atEOF is true, the value of c is
    // irrelevant.

    fun step(c: Char, atEOF: Boolean) {
        when (state) {

            // ── FIELD_START ──────────────────────────────────────────────────
            ParseState.FIELD_START -> {
                if (atEOF) {
                    // EOF at field_start: flush if we have an in-progress row.
                    // Trailing newline with no content → don't emit empty row.
                    if (currentRow.isNotEmpty()) flushRow()
                    return
                }

                when {
                    c == '"' -> {
                        // Opening double-quote: enter quoted mode.
                        // The '"' is structural — don't add it to the buffer.
                        state = ParseState.IN_QUOTED_FIELD
                    }
                    c == delim -> {
                        // Delimiter at field start: current field is empty ("").
                        finishField()
                        // state stays at FIELD_START
                    }
                    c == '\r' -> handleCR()
                    c == '\n' -> {
                        if (currentRow.isNotEmpty()) {
                            // '\n' with a non-empty row: last field before
                            // this newline was empty (delimiter immediately
                            // before newline). Emit "" and complete the row.
                            currentRow.add("")
                            flushRow()
                        }
                        // Empty currentRow → blank line, do nothing.
                    }
                    else -> {
                        // Any other character: start of an unquoted field.
                        fieldBuf.append(c)
                        state = ParseState.IN_UNQUOTED_FIELD
                    }
                }
            }

            // ── IN_UNQUOTED_FIELD ────────────────────────────────────────────
            ParseState.IN_UNQUOTED_FIELD -> {
                if (atEOF) {
                    finishField()
                    flushRow()
                    return
                }

                when {
                    c == delim -> {
                        finishField()
                        state = ParseState.FIELD_START
                    }
                    c == '\r' -> {
                        finishField()
                        flushRow()
                        handleCR()
                        state = ParseState.FIELD_START
                    }
                    c == '\n' -> {
                        finishField()
                        flushRow()
                        state = ParseState.FIELD_START
                    }
                    else -> fieldBuf.append(c)
                }
            }

            // ── IN_QUOTED_FIELD ──────────────────────────────────────────────
            ParseState.IN_QUOTED_FIELD -> {
                if (atEOF) {
                    // EOF inside a quoted field: malformed input.
                    throw CsvParseException("unclosed quoted field at end of input")
                }

                if (c == '"') {
                    // Could be end-of-field or start of "" escape.
                    state = ParseState.IN_QUOTED_MAYBE_END
                } else {
                    // Literal character (including commas, newlines, etc.)
                    fieldBuf.append(c)
                }
            }

            // ── IN_QUOTED_MAYBE_END ──────────────────────────────────────────
            ParseState.IN_QUOTED_MAYBE_END -> {
                if (atEOF) {
                    // EOF after closing '"': field ended cleanly.
                    finishField()
                    flushRow()
                    return
                }

                when {
                    c == '"' -> {
                        // Another '"': this is a "" escape.
                        // Append one literal '"' and return to IN_QUOTED_FIELD.
                        //
                        // "say ""hello""" → say "hello"
                        //   After opening '"':      state=IN_QUOTED
                        //   'say ':                 buf="say "
                        //   First '"' of "":        state=IN_QUOTED_MAYBE_END
                        //   Second '"' of "":       state=IN_QUOTED, buf="say \""
                        //   'hello':                buf="say \"hello"
                        //   First '"' of "":        state=IN_QUOTED_MAYBE_END
                        //   Second '"' of "":       state=IN_QUOTED, buf="say \"hello\""
                        //   Final '"':              state=IN_QUOTED_MAYBE_END
                        //   EOF/delim:              emit  say "hello"
                        fieldBuf.append('"')
                        state = ParseState.IN_QUOTED_FIELD
                    }
                    c == delim -> {
                        finishField()
                        state = ParseState.FIELD_START
                    }
                    c == '\r' -> {
                        finishField()
                        flushRow()
                        handleCR()
                        state = ParseState.FIELD_START
                    }
                    c == '\n' -> {
                        finishField()
                        flushRow()
                        state = ParseState.FIELD_START
                    }
                    else -> {
                        // Any other character: malformed per RFC 4180.
                        // We tolerate it: end the quoted field and re-process
                        // this character from FIELD_START on the next step.
                        //
                        // Decrement pos so the main loop re-increments it and
                        // feeds this character again (re-process pattern).
                        finishField()
                        state = ParseState.FIELD_START
                        pos-- // re-process this character
                    }
                }
            }
        }
    }

    // ==========================================================================
    // Helper methods
    // ==========================================================================

    /**
     * Completes the current field by appending the buffer contents to
     * [currentRow] and resetting the buffer. An empty buffer produces `""`.
     */
    private fun finishField() {
        currentRow.add(fieldBuf.toString())
        fieldBuf.clear()
    }

    /**
     * Completes the current row by appending a snapshot of [currentRow] to
     * [rows] and clearing [currentRow].
     */
    private fun flushRow() {
        rows.add(currentRow.toList()) // immutable snapshot
        currentRow.clear()
    }

    /**
     * Handles a carriage return (`'\r'`). If the next character is `'\n'`
     * (forming a Windows `\r\n` pair), we advance past it so the main loop
     * doesn't double-process the newline.
     */
    private fun handleCR() {
        val next = pos + 1
        if (next < chars.size && chars[next] == '\n') {
            pos++ // skip the '\n' of the \r\n pair
        }
    }

    /** Returns the first row (header), or `null` if no rows were parsed. */
    fun header(): List<String>? = rows.firstOrNull()

    /** Returns all rows after the first, or an empty list if fewer than 2 rows. */
    fun dataRows(): List<List<String>> = if (rows.size <= 1) emptyList() else rows.drop(1)
}

// ============================================================================
// buildRowMaps — zip data rows with header to produce List<Map<String,String>>
// ============================================================================
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
// We use LinkedHashMap (via Kotlin's `linkedMapOf()`) to preserve column
// insertion order in the output maps.

private fun buildRowMaps(
    header: List<String>?,
    dataRows: List<List<String>>
): List<Map<String, String>> {
    if (header == null || dataRows.isEmpty()) return emptyList()

    return dataRows.map { row ->
        buildMap(header.size) {
            for (i in header.indices) {
                put(header[i], if (i < row.size) row[i] else "")
            }
            // Fields beyond header.size are silently dropped (long rows).
        }
    }
}
