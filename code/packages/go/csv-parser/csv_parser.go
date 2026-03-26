// Package csvparser implements a CSV parser following a pragmatic superset of
// RFC 4180 semantics — entirely from scratch, without using encoding/csv or
// any other standard-library CSV facilities.
//
// # What is CSV?
//
// CSV (Comma-Separated Values) is the world's most common data interchange
// format. Spreadsheets export it, databases dump it, scientists share it.
// Despite its ubiquity, CSV has no single standard. RFC 4180 (2005) is the
// closest thing, but real-world files deviate from it constantly.
//
// This package implements a pragmatic dialect:
//
//   - First row is always the header (defines column names)
//   - All values returned as strings — no type coercion
//   - Quoted fields can contain commas, newlines, and "" (escaped double-quote)
//   - Configurable delimiter (default: comma)
//   - Ragged rows: short rows padded with "", long rows truncated
//   - Unclosed quoted field → error
//
// # Why a Hand-Rolled Parser?
//
// CSV cannot be tokenized with a simple regex. Consider:
//
//	field1,"field,with,commas",field3
//
// The commas inside the quoted field are not delimiters — but the parser can
// only know that after entering quoted mode. This context-sensitivity means
// CSV parsers are typically hand-rolled character-by-character state machines,
// not grammar-driven lexer/parser pipelines.
//
// Think of it like reading aloud: when you see a '"', you enter "quoted mode"
// and treat everything differently until the closing '"'.
//
// # The State Machine
//
// The parser uses exactly four states:
//
//	                 ┌──────────────┐
//	                 │  FIELD_START │◄──────────────────────────────────┐
//	                 └──────┬───────┘                                   │
//	                        │                                           │
//	           ┌────────────┼──────────────────┐                        │
//	           │            │                  │                        │
//	          '"'        other char      DELIMITER or NEWLINE           │
//	           │            │                  │                        │
//	           ▼            ▼                  │                        │
//	    ┌────────────┐  ┌────────────┐          │  emit empty field      │
//	    │  IN_QUOTED  │  │IN_UNQUOTED│          └───────────────────────┘
//	    │   _FIELD    │  │  _FIELD   │
//	    └──────┬──────┘  └─────┬─────┘
//	           │               │
//	     '"'   │          DELIMITER → end field
//	           │          NEWLINE   → end row
//	           ▼          EOF       → end file
//	  ┌──────────────────┐
//	  │IN_QUOTED_MAYBE_END│
//	  └──────┬────────────┘
//	         │
//	    ┌────┴────┐
//	   '"'    DELIMITER/NEWLINE/EOF
//	    │          │
//	 escaped    end field
//	  quote
//	 append '"'
//	 back to IN_QUOTED_FIELD
//
// # Truth Table: IN_QUOTED_MAYBE_END Transitions
//
//	Previous | Next char    | Interpretation          | Action
//	char     |              |                         |
//	─────────┼──────────────┼─────────────────────────┼─────────────────────
//	   "     | "            | escaped quote ("")      | emit '"', stay quoted
//	   "     | DELIMITER    | end of quoted field     | emit field, next field
//	   "     | \n or \r     | end of quoted field     | emit field, next row
//	   "     | EOF          | end of quoted field     | emit field, end file
//	   "     | other        | malformed (tolerant)    | end field, re-process
package csvparser

import (
	"fmt"
	"strings"
)

// ---------------------------------------------------------------------------
// ParseState — the four states of the CSV state machine
// ---------------------------------------------------------------------------
//
// Using a named type with iota constants makes the code self-documenting.
// Pattern-matching on ParseState in parseStep makes the state machine
// transitions easy to read and verify against the spec.

// ParseState represents the current state of the CSV parser's state machine.
// The parser is always in exactly one of these four states.
type ParseState int

const (
	// StateFieldStart is the initial state and the state we return to after
	// finishing each field. In this state we are about to read the first
	// character of a new field.
	//
	// Transitions:
	//   '"'         → StateInQuotedField  (opening quote: enter quoted mode)
	//   delimiter   → StateFieldStart     (empty field; emit "", start next)
	//   '\n'/'\r'   → StateFieldStart     (end of row)
	//   EOF         → done
	//   other       → StateInUnquotedField
	StateFieldStart ParseState = iota

	// StateInUnquotedField is the state for collecting a plain (unquoted)
	// field. We stay in this state until we see a delimiter, newline, or EOF.
	//
	// Transitions:
	//   delimiter   → StateFieldStart     (end field, start next)
	//   '\n'/'\r'   → StateFieldStart     (end field, end row)
	//   EOF         → done
	//   other       → StateInUnquotedField (keep collecting)
	StateInUnquotedField

	// StateInQuotedField is the state for collecting a quoted field. Inside
	// a quoted field, almost everything is literal — commas, newlines, etc.
	// Only '"' is special.
	//
	// Transitions:
	//   '"'         → StateInQuotedMaybeEnd
	//   EOF         → error (unclosed quote)
	//   other       → StateInQuotedField (keep collecting)
	StateInQuotedField

	// StateInQuotedMaybeEnd is the state after seeing '"' inside a quoted
	// field. The next character determines whether this was:
	//   - A "" escape (next char is '"'): append '"' to buffer, back to quoted
	//   - End of the quoted field (anything else)
	//
	// Transitions:
	//   '"'         → StateInQuotedField  (escape: emit '"', stay in quotes)
	//   delimiter   → StateFieldStart     (end field, next field)
	//   '\n'/'\r'   → StateFieldStart     (end field, end row)
	//   EOF         → done (field ended cleanly)
	//   other       → StateFieldStart     (tolerant: end field, re-process char)
	StateInQuotedMaybeEnd
)

// ---------------------------------------------------------------------------
// CsvError — error type for malformed CSV
// ---------------------------------------------------------------------------

// CsvError is returned when the parser encounters malformed CSV input.
// Currently this only occurs for unclosed quoted fields.
type CsvError struct {
	// Message describes what went wrong.
	Message string
}

// Error implements the error interface.
func (e *CsvError) Error() string {
	return fmt.Sprintf("csv parse error: %s", e.Message)
}

// ---------------------------------------------------------------------------
// parser — internal parser state
// ---------------------------------------------------------------------------
//
// We encapsulate all mutable state into a struct rather than passing it
// through function parameters. This is idiomatic Go for a hand-rolled parser:
// method calls on *parser update the state in place, keeping the top-level
// Parse functions clean.

// parser holds the mutable state of an in-progress CSV parse.
type parser struct {
	// runes is the input as a slice of runes (Unicode code points).
	// Using []rune rather than []byte ensures we handle multi-byte UTF-8
	// characters correctly. A single Chinese character, for example, is
	// one rune but three bytes.
	runes []rune

	// pos is the current read position within runes.
	pos int

	// state is the current state machine state.
	state ParseState

	// fieldBuf accumulates characters for the current field.
	// We use strings.Builder for efficient string construction.
	// Builder grows the internal buffer as needed (like a dynamic array),
	// so appending one character at a time is O(n) total, not O(n²).
	fieldBuf strings.Builder

	// currentRow accumulates completed field strings for the current row.
	currentRow []string

	// rows accumulates completed rows. Each row is a []string of field values.
	rows [][]string

	// delim is the delimiter rune (e.g., ',', '\t', ';').
	delim rune
}

// newParser creates a fresh parser for the given input and delimiter.
func newParser(source string, delimiter rune) *parser {
	return &parser{
		runes: []rune(source),
		pos:   0,
		state: StateFieldStart,
		delim: delimiter,
	}
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// ParseCSV parses CSV text using the default comma delimiter.
//
// Returns a slice of maps from column name to string value on success.
// The first row of the input is treated as the header and defines the map
// keys; it does not appear in the returned slice.
//
// Returns an error if the input contains an unclosed quoted field.
//
// Edge cases:
//   - Empty input       → nil (empty slice), no error
//   - Header-only input → nil (empty slice), no error
//   - Ragged rows       → short rows padded with "", long rows truncated
//
// Example:
//
//	rows, err := ParseCSV("name,age\nAlice,30\nBob,25\n")
//	// rows[0]["name"] == "Alice"
//	// rows[0]["age"]  == "30"
func ParseCSV(source string) ([]map[string]string, error) {
	return ParseCSVWithDelimiter(source, ',')
}

// ParseCSVWithDelimiter is like ParseCSV but with a configurable delimiter.
//
// The delimiter parameter is a single rune. Common choices:
//   - ','  — comma (the default; use ParseCSV)
//   - '\t' — tab (TSV / tab-separated values)
//   - ';'  — semicolon (common in European locales)
//   - '|'  — pipe-separated
//
// Example:
//
//	rows, err := ParseCSVWithDelimiter("name\tage\nAlice\t30\n", '\t')
func ParseCSVWithDelimiter(source string, delimiter rune) ([]map[string]string, error) {
	p := newParser(source, delimiter)

	if err := p.run(); err != nil {
		return nil, err
	}

	return buildRowMaps(p.header(), p.dataRows()), nil
}

// ---------------------------------------------------------------------------
// parser.run — drives the state machine
// ---------------------------------------------------------------------------
//
// run processes the entire input by stepping the state machine one rune at a
// time. At EOF it flushes any in-progress field and row.
//
// The design is a simple for loop over the rune slice rather than a recursive
// descent, because Go's stack is not as naturally recursive as Elixir's. The
// state machine's logic is encoded in a switch statement on (state, rune).

func (p *parser) run() error {
	for p.pos <= len(p.runes) {
		var r rune
		atEOF := p.pos == len(p.runes)

		if !atEOF {
			r = p.runes[p.pos]
		}

		if err := p.step(r, atEOF); err != nil {
			return err
		}

		if atEOF {
			break
		}

		p.pos++
	}

	return nil
}

// step processes a single rune (or EOF) and updates the parser state.
//
// The logic is a large switch on (p.state, rune) that mirrors the state
// machine diagram in the package comment. Each case is documented with
// the transition it represents.
//
// When atEOF is true, the value of r is ignored — we're signalling that
// the input has ended and any in-progress field/row should be flushed.
func (p *parser) step(r rune, atEOF bool) error {
	// ── FIELD_START ──────────────────────────────────────────────────────
	if p.state == StateFieldStart {
		if atEOF {
			// EOF while at field_start: if we have an in-progress row (meaning
			// there was real content before this), flush it. If current_row is
			// empty, this is a trailing newline — don't emit a spurious empty row.
			if len(p.currentRow) > 0 {
				p.flushRow()
			}
			return nil
		}

		switch {
		case r == '"':
			// Opening double-quote: enter quoted mode.
			// Don't add the '"' to the field buffer — it's a structural character,
			// not part of the field value.
			p.state = StateInQuotedField

		case r == p.delim:
			// Delimiter at field start: the current field is empty ("").
			// Example: in `a,,b`, the second ',' arrives here.
			p.finishField()
			// state stays at StateFieldStart for the next field

		case r == '\r':
			// '\r' at field_start: skip it (handled as part of \r\n or alone).
			// Peek ahead to consume the '\n' in \r\n pairs.
			p.handleCR()

		case r == '\n':
			// '\n' at field_start with a non-empty row: the last field before
			// this newline was empty (delimiter immediately before newline).
			// Emit empty field and complete the row.
			//
			// With an empty row: this is a blank line — skip it.
			if len(p.currentRow) > 0 {
				p.currentRow = append(p.currentRow, "")
				p.flushRow()
			}
			// If currentRow is empty: blank line, do nothing.

		default:
			// Any other character: start of an unquoted field.
			p.fieldBuf.WriteRune(r)
			p.state = StateInUnquotedField
		}

		return nil
	}

	// ── IN_UNQUOTED_FIELD ────────────────────────────────────────────────
	if p.state == StateInUnquotedField {
		if atEOF {
			// EOF while collecting an unquoted field: flush field and row.
			p.finishField()
			p.flushRow()
			return nil
		}

		switch {
		case r == p.delim:
			// Delimiter: end of this field.
			p.finishField()
			p.state = StateFieldStart

		case r == '\r':
			// '\r' triggers end-of-row (handle \r\n or bare \r).
			p.finishField()
			p.flushRow()
			p.handleCR()
			p.state = StateFieldStart

		case r == '\n':
			// '\n': end of row.
			p.finishField()
			p.flushRow()
			p.state = StateFieldStart

		default:
			// Literal character: add to field buffer.
			p.fieldBuf.WriteRune(r)
		}

		return nil
	}

	// ── IN_QUOTED_FIELD ──────────────────────────────────────────────────
	if p.state == StateInQuotedField {
		if atEOF {
			// EOF inside a quoted field: this is malformed input.
			// RFC 4180 requires every quoted field to have a closing '"'.
			return &CsvError{Message: "unclosed quoted field at end of input"}
		}

		if r == '"' {
			// '"' inside a quoted field: might be the end of the field, or
			// it might be the first '"' of a "" escape. Transition to
			// IN_QUOTED_MAYBE_END to look at the next character.
			p.state = StateInQuotedMaybeEnd
		} else {
			// Everything else (including commas, newlines, backslashes) is
			// a literal character inside a quoted field.
			p.fieldBuf.WriteRune(r)
		}

		return nil
	}

	// ── IN_QUOTED_MAYBE_END ──────────────────────────────────────────────
	if p.state == StateInQuotedMaybeEnd {
		if atEOF {
			// EOF after a closing '"': the quoted field ended cleanly.
			// Flush the field and row.
			p.finishField()
			p.flushRow()
			return nil
		}

		switch {
		case r == '"':
			// Another '"': this is a "" escape. Append one literal '"' to
			// the buffer and return to IN_QUOTED_FIELD.
			//
			// Example: "say ""hello""" → say "hello"
			//
			// Walking through `"say ""hello"""`:
			//   After opening '"':   state=IN_QUOTED
			//   After 's','a','y',' ': state=IN_QUOTED, buf="say "
			//   First '"' of "":    state=IN_QUOTED_MAYBE_END
			//   Second '"' of "":   state=IN_QUOTED, buf=`say "`
			//   After 'h','e','l','l','o': buf=`say "hello`
			//   First '"' of "":    state=IN_QUOTED_MAYBE_END
			//   Second '"' of "":   state=IN_QUOTED, buf=`say "hello"`
			//   Final '"':          state=IN_QUOTED_MAYBE_END
			//   EOF/delim:          emit field `say "hello"`
			p.fieldBuf.WriteRune('"')
			p.state = StateInQuotedField

		case r == p.delim:
			// Delimiter after closing '"': end of quoted field.
			p.finishField()
			p.state = StateFieldStart

		case r == '\r':
			// '\r' after closing '"': end of row.
			p.finishField()
			p.flushRow()
			p.handleCR()
			p.state = StateFieldStart

		case r == '\n':
			// '\n' after closing '"': end of row.
			p.finishField()
			p.flushRow()
			p.state = StateFieldStart

		default:
			// Any other character after '"': malformed per RFC 4180, but we
			// tolerate it. End the quoted field and re-process this character
			// from StateFieldStart on the next step.
			//
			// To re-process: we finish the field, set state to FIELD_START,
			// and then *don't* advance pos — so the same rune is seen again
			// on the next iteration. We achieve this by decrementing pos by 1,
			// which will be re-incremented by the main loop.
			p.finishField()
			p.state = StateFieldStart
			p.pos-- // re-process this character
		}

		return nil
	}

	return nil
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

// finishField completes the current field by taking the contents of fieldBuf,
// appending the resulting string to currentRow, and resetting the buffer.
//
// The field value is the exact contents of fieldBuf — no trimming.
// An empty buffer produces the empty string "".
func (p *parser) finishField() {
	p.currentRow = append(p.currentRow, p.fieldBuf.String())
	p.fieldBuf.Reset()
}

// flushRow completes the current row by appending currentRow to rows, then
// resetting currentRow to an empty slice.
//
// If currentRow is empty we still flush it — the caller is responsible for
// deciding whether to call flushRow for empty rows.
//
// In practice, we only call flushRow when we know there is actual row content
// (newline after a non-empty row).
func (p *parser) flushRow() {
	// Make a copy of the slice, because currentRow will be reused.
	row := make([]string, len(p.currentRow))
	copy(row, p.currentRow)
	p.rows = append(p.rows, row)
	p.currentRow = p.currentRow[:0]
}

// handleCR handles a carriage return character. If the next rune is '\n'
// (Windows \r\n line ending), we advance past it. This way the main loop
// never sees the '\n' of a \r\n pair and won't double-process it.
//
// Called when we see '\r' and have already decided to end the current row.
func (p *parser) handleCR() {
	// Check if the next character is '\n' (the '\n' of a \r\n pair).
	next := p.pos + 1
	if next < len(p.runes) && p.runes[next] == '\n' {
		// Advance past the '\n' so the main loop skips it.
		p.pos++
	}
}

// header returns the first row (the header row).
// Returns nil if there are no rows at all.
func (p *parser) header() []string {
	if len(p.rows) == 0 {
		return nil
	}
	return p.rows[0]
}

// dataRows returns all rows after the first (the data rows).
// Returns nil if there are fewer than 2 rows.
func (p *parser) dataRows() [][]string {
	if len(p.rows) <= 1 {
		return nil
	}
	return p.rows[1:]
}

// ---------------------------------------------------------------------------
// buildRowMaps — convert row slices to maps using the header as keys
// ---------------------------------------------------------------------------
//
// buildRowMaps zips each data row with the header to produce a slice of maps.
//
// Handles ragged rows:
//
//	Short row: pad missing fields with ""
//
//	  header: ["a", "b", "c"]
//	  row:    ["1", "2"]         ← missing "c"
//	  result: {"a":"1","b":"2","c":""}
//
//	Long row: truncate extra fields
//
//	  header: ["a", "b"]
//	  row:    ["1", "2", "3"]    ← extra "3" discarded
//	  result: {"a":"1","b":"2"}
//
// Why not error on ragged rows? Many real-world CSV generators are buggy and
// produce inconsistent column counts. The spec says: use the header as the
// authoritative column list; pad or truncate data rows to match.
func buildRowMaps(header []string, dataRows [][]string) []map[string]string {
	if len(header) == 0 || len(dataRows) == 0 {
		return nil
	}

	result := make([]map[string]string, 0, len(dataRows))

	for _, row := range dataRows {
		m := make(map[string]string, len(header))

		for i, col := range header {
			if i < len(row) {
				// Field exists in this row: use it.
				m[col] = row[i]
			} else {
				// Field is missing from this row (short row): pad with "".
				m[col] = ""
			}
		}
		// Fields beyond len(header) are silently ignored (long row truncation).

		result = append(result, m)
	}

	return result
}
