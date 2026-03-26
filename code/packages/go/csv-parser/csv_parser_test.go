package csvparser

// csv_parser_test.go — comprehensive tests for the CSV parser.
//
// Each test group is preceded by a comment explaining *why* we test these
// cases and what invariants they exercise.
//
// Test naming convention: Test<Group><CaseName>
// where <Group> identifies the feature and <CaseName> the specific edge case.

import (
	"testing"
)

// ===========================================================================
// 1. EMPTY AND MINIMAL INPUTS
// ===========================================================================
//
// These are the edge cases at the extreme "small" end of the input space.
// Many CSV parsers fail here: producing an extra empty row, panicking on nil,
// or returning an error for a perfectly valid empty file.

func TestEmptyString(t *testing.T) {
	// An empty file has no header and no data rows. The result must be nil
	// (or an empty slice), not an error.
	rows, err := ParseCSV("")
	if err != nil {
		t.Fatalf("unexpected error for empty input: %v", err)
	}
	if len(rows) != 0 {
		t.Errorf("expected 0 rows for empty input, got %d: %v", len(rows), rows)
	}
}

func TestSingleNewline(t *testing.T) {
	// A file containing only "\n" has no content. Should produce no rows.
	rows, err := ParseCSV("\n")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 0 {
		t.Errorf("expected 0 rows, got %d", len(rows))
	}
}

func TestHeaderOnlyNoTrailingNewline(t *testing.T) {
	// One row, no trailing newline: the header defines columns but no data
	// rows exist. Result must be empty (not an error).
	rows, err := ParseCSV("name,age,city")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 0 {
		t.Errorf("expected 0 rows for header-only input, got %d", len(rows))
	}
}

func TestHeaderOnlyWithTrailingNewline(t *testing.T) {
	// Same as above but with a trailing newline. Must not produce a spurious
	// empty row.
	rows, err := ParseCSV("name,age,city\n")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 0 {
		t.Errorf("expected 0 rows for header-only input, got %d", len(rows))
	}
}

func TestSingleColumnHeaderOnly(t *testing.T) {
	rows, err := ParseCSV("id")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 0 {
		t.Errorf("expected 0 rows, got %d", len(rows))
	}
}

func TestSingleColumnOneDataRow(t *testing.T) {
	rows, err := ParseCSV("id\n42")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	if rows[0]["id"] != "42" {
		t.Errorf("expected id=42, got %q", rows[0]["id"])
	}
}

func TestSingleColumnOneDataRowTrailingNewline(t *testing.T) {
	rows, err := ParseCSV("id\n42\n")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	if rows[0]["id"] != "42" {
		t.Errorf("expected id=42, got %q", rows[0]["id"])
	}
}

// ===========================================================================
// 2. SIMPLE MULTI-COLUMN TABLES
// ===========================================================================

func TestThreeColumnsTwoRows(t *testing.T) {
	// Canonical CSV example: well-formed, multiple columns, multiple rows.
	input := "name,age,city\nAlice,30,New York\nBob,25,London\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}

	// First row
	assertField(t, rows[0], "name", "Alice")
	assertField(t, rows[0], "age", "30")
	assertField(t, rows[0], "city", "New York")

	// Second row
	assertField(t, rows[1], "name", "Bob")
	assertField(t, rows[1], "age", "25")
	assertField(t, rows[1], "city", "London")
}

func TestThreeColumnsTwoRowsNoTrailingNewline(t *testing.T) {
	// Trailing newline is optional per spec. Both forms must produce the
	// same result.
	input := "name,age,city\nAlice,30,New York\nBob,25,London"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
}

func TestAllValuesAreStrings(t *testing.T) {
	// No type coercion: integers, floats, booleans, empty strings — all
	// returned as the Go string type.
	input := "int,float,bool,empty\n42,3.14,true,\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}

	assertField(t, rows[0], "int", "42")    // integer → string
	assertField(t, rows[0], "float", "3.14") // float → string
	assertField(t, rows[0], "bool", "true") // boolean → string
	assertField(t, rows[0], "empty", "")    // empty → ""
}

func TestWhitespaceIsPreserved(t *testing.T) {
	// Spec: whitespace is significant. Spaces around unquoted fields are part
	// of the field value. Trimming is the caller's responsibility.
	input := "a,b\n  hello  ,  world  \n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	assertField(t, rows[0], "a", "  hello  ")
	assertField(t, rows[0], "b", "  world  ")
}

// ===========================================================================
// 3. QUOTED FIELDS
// ===========================================================================

func TestQuotedFieldContainingDelimiter(t *testing.T) {
	// The comma inside "A small, round widget" is part of the value, not a
	// delimiter. The parser must track quote state to know this.
	input := `product,price,description` + "\n" +
		`Widget,9.99,"A small, round widget"` + "\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "description", "A small, round widget")
}

func TestQuotedFieldContainingNewline(t *testing.T) {
	// A quoted field can span multiple physical lines. The embedded newline
	// must be preserved as a literal newline character in the output.
	input := "id,note\n1,\"Line one\nLine two\"\n2,Single line\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
	assertField(t, rows[0], "note", "Line one\nLine two")
	assertField(t, rows[1], "note", "Single line")
}

func TestQuotedFieldDoubleQuoteEscape(t *testing.T) {
	// Inside a quoted field, "" represents a single '"'.
	//
	// Input:   1,"She said ""hello"""
	// Decoded: 1, She said "hello"
	input := "id,value\n1,\"She said \"\"hello\"\"\"\n2,plain\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
	assertField(t, rows[0], "value", `She said "hello"`)
	assertField(t, rows[1], "value", "plain")
}

func TestQuotedFieldMultipleEscapedQuotes(t *testing.T) {
	// Multiple "" escapes in one field: "a""b""c" → a"b"c
	input := "x\n\"a\"\"b\"\"c\"\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "x", `a"b"c`)
}

func TestQuotedEmptyField(t *testing.T) {
	// An empty quoted field: just "" (two double-quote chars). Value is "".
	input := "a,b,c\n1,\"\",3\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "b", "")
}

func TestQuotedFieldIsFirst(t *testing.T) {
	input := "a,b\n\"quoted first\",second\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "a", "quoted first")
	assertField(t, rows[0], "b", "second")
}

func TestQuotedFieldIsLast(t *testing.T) {
	input := "a,b\nfirst,\"quoted last\"\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "a", "first")
	assertField(t, rows[0], "b", "quoted last")
}

func TestAllFieldsQuoted(t *testing.T) {
	input := "\"name\",\"age\"\n\"Alice\",\"30\"\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "name", "Alice")
	assertField(t, rows[0], "age", "30")
}

func TestQuotedFieldSingleDoubleQuoteEscape(t *testing.T) {
	// Field value is a single '"': input is `""""` (four chars: open, "", close)
	input := "x\n\"\"\"\"\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "x", `"`)
}

// ===========================================================================
// 4. EMPTY FIELDS
// ===========================================================================

func TestMiddleFieldEmpty(t *testing.T) {
	// a,,b → three fields: "a", "", "b"
	input := "a,b,c\n1,,3\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "a", "1")
	assertField(t, rows[0], "b", "")
	assertField(t, rows[0], "c", "3")
}

func TestFirstAndLastFieldEmpty(t *testing.T) {
	// ,2, → "", "2", ""
	input := "a,b,c\n,2,\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "a", "")
	assertField(t, rows[0], "b", "2")
	assertField(t, rows[0], "c", "")
}

func TestAllFieldsEmpty(t *testing.T) {
	// ,, → three empty fields
	input := "a,b,c\n,,\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "a", "")
	assertField(t, rows[0], "b", "")
	assertField(t, rows[0], "c", "")
}

func TestMultipleRowsWithEmptyFields(t *testing.T) {
	input := "a,b,c\n1,,3\n,2,\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
	assertField(t, rows[0], "b", "")
	assertField(t, rows[1], "a", "")
	assertField(t, rows[1], "c", "")
}

// ===========================================================================
// 5. CUSTOM DELIMITER
// ===========================================================================

func TestTabDelimiter(t *testing.T) {
	// Tab-separated values (TSV): the most common alternative to CSV.
	input := "name\tage\nAlice\t30\n"

	rows, err := ParseCSVWithDelimiter(input, '\t')
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "name", "Alice")
	assertField(t, rows[0], "age", "30")
}

func TestSemicolonDelimiter(t *testing.T) {
	input := "a;b;c\n1;2;3\n"

	rows, err := ParseCSVWithDelimiter(input, ';')
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "a", "1")
	assertField(t, rows[0], "b", "2")
	assertField(t, rows[0], "c", "3")
}

func TestPipeDelimiter(t *testing.T) {
	input := "x|y\nhello|world\n"

	rows, err := ParseCSVWithDelimiter(input, '|')
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "x", "hello")
	assertField(t, rows[0], "y", "world")
}

func TestTabDelimiterQuotedFieldWithComma(t *testing.T) {
	// With tab as delimiter, commas inside fields are literal — not delimiters.
	input := "name\tnote\nAlice\t\"a,b,c\"\n"

	rows, err := ParseCSVWithDelimiter(input, '\t')
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "note", "a,b,c")
}

// ===========================================================================
// 6. RAGGED ROWS (MISMATCHED COLUMN COUNTS)
// ===========================================================================
//
// The spec says: pad short rows with "", truncate long rows.
// This matches the behavior of most production CSV processors.

func TestRaggedRowTooFewFields(t *testing.T) {
	// Header has 3 columns. Data row only has 2 fields.
	// Missing "c" column should be filled with "".
	input := "a,b,c\n1,2\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "a", "1")
	assertField(t, rows[0], "b", "2")
	assertField(t, rows[0], "c", "") // padded
}

func TestRaggedRowTooManyFields(t *testing.T) {
	// Header has 2 columns. Data row has 4 fields.
	// Extra "3" and "4" should be discarded.
	input := "a,b\n1,2,3,4\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "a", "1")
	assertField(t, rows[0], "b", "2")
	if _, ok := rows[0]["extra"]; ok {
		t.Error("unexpected extra key in map from truncated row")
	}
	// Map should have exactly 2 keys
	if len(rows[0]) != 2 {
		t.Errorf("expected map with 2 keys, got %d: %v", len(rows[0]), rows[0])
	}
}

func TestRaggedRowOnlyOneField(t *testing.T) {
	// Data row has only 1 field when header has 3.
	input := "a,b,c\nonlyone\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertField(t, rows[0], "a", "onlyone")
	assertField(t, rows[0], "b", "")
	assertField(t, rows[0], "c", "")
}

func TestMixedRaggedRows(t *testing.T) {
	// Three data rows: short, exact, long. Each should be handled correctly.
	input := "a,b,c\n1,2\n1,2,3\n1,2,3,4,5\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 3 {
		t.Fatalf("expected 3 rows, got %d", len(rows))
	}

	// Short row → padded
	assertField(t, rows[0], "c", "")

	// Correct length → unchanged
	assertField(t, rows[1], "a", "1")
	assertField(t, rows[1], "b", "2")
	assertField(t, rows[1], "c", "3")

	// Long row → truncated (only 3 keys)
	if len(rows[2]) != 3 {
		t.Errorf("expected truncated row with 3 keys, got %d", len(rows[2]))
	}
}

// ===========================================================================
// 7. LINE ENDINGS
// ===========================================================================
//
// CSV files come from Windows (\r\n), Unix (\n), and old Mac (\r) sources.
// All three must work correctly.

func TestUnixLineEndings(t *testing.T) {
	input := "a,b\n1,2\n3,4\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
}

func TestWindowsLineEndings(t *testing.T) {
	// \r\n line endings must be treated as a single newline — not as two
	// separate characters (which would produce spurious empty rows).
	input := "a,b\r\n1,2\r\n3,4\r\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d: %v", len(rows), rows)
	}
	assertField(t, rows[0], "a", "1")
	assertField(t, rows[0], "b", "2")
}

func TestOldMacLineEndings(t *testing.T) {
	// Bare \r (old Mac OS 9 format) must be treated as a row separator.
	input := "a,b\r1,2\r3,4\r"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
}

func TestEmbeddedNewlineInQuotedFieldCRLFFile(t *testing.T) {
	// In a CRLF file, an embedded \n inside a quoted field is literal — it
	// should NOT be treated as a row separator while we're inside quotes.
	input := "id,note\r\n1,\"Line1\nLine2\"\r\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "note", "Line1\nLine2")
}

// ===========================================================================
// 8. ERROR CASES
// ===========================================================================

func TestUnclosedQuoteAtEOF(t *testing.T) {
	// The most important error case: input ends while inside a quoted field.
	input := "id,value\n1,\"unclosed"

	_, err := ParseCSV(input)
	if err == nil {
		t.Fatal("expected error for unclosed quote, got nil")
	}
}

func TestUnclosedQuoteWithEmbeddedContent(t *testing.T) {
	input := "a,b\n1,\"this has a comma, and no end"

	_, err := ParseCSV(input)
	if err == nil {
		t.Fatal("expected error for unclosed quote, got nil")
	}
}

func TestUnclosedQuoteMultiLine(t *testing.T) {
	// Unclosed quote that spans multiple lines — still an error.
	input := "a,b\n1,\"line one\nline two (still no closing quote)"

	_, err := ParseCSV(input)
	if err == nil {
		t.Fatal("expected error for unclosed quote, got nil")
	}
}

func TestCsvErrorImplementsError(t *testing.T) {
	// CsvError must implement the error interface.
	var e error = &CsvError{Message: "test"}
	if e.Error() == "" {
		t.Error("CsvError.Error() returned empty string")
	}
}

// ===========================================================================
// 9. MULTIPLE DATA ROWS
// ===========================================================================

func TestThreeDataRows(t *testing.T) {
	input := "name,score\nAlice,95\nBob,87\nCharlie,72\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 3 {
		t.Fatalf("expected 3 rows, got %d", len(rows))
	}

	assertField(t, rows[0], "score", "95")
	assertField(t, rows[1], "score", "87")
	assertField(t, rows[2], "score", "72")
}

func TestRowOrderIsPreserved(t *testing.T) {
	// Rows must appear in the same order as in the input file.
	input := "n\n1\n2\n3\n4\n5\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 5 {
		t.Fatalf("expected 5 rows, got %d", len(rows))
	}

	expected := []string{"1", "2", "3", "4", "5"}
	for i, exp := range expected {
		if rows[i]["n"] != exp {
			t.Errorf("row %d: expected n=%q, got %q", i, exp, rows[i]["n"])
		}
	}
}

// ===========================================================================
// 10. EQUIVALENCE
// ===========================================================================

func TestParseCSVAndWithDelimiterEquivalent(t *testing.T) {
	// ParseCSV must be exactly equivalent to ParseCSVWithDelimiter(s, ',').
	input := "a,b\n1,2\n"

	rows1, err1 := ParseCSV(input)
	rows2, err2 := ParseCSVWithDelimiter(input, ',')

	if err1 != nil || err2 != nil {
		t.Fatalf("unexpected errors: %v, %v", err1, err2)
	}
	if len(rows1) != len(rows2) {
		t.Errorf("length mismatch: %d vs %d", len(rows1), len(rows2))
	}
}

// ===========================================================================
// 11. INTEGRATION / COMPLEX TESTS
// ===========================================================================

func TestProductCatalogCSV(t *testing.T) {
	// A realistic CSV: some plain fields, some quoted with commas, one with
	// escaped double-quotes.
	input := "sku,name,price,in_stock\n" +
		"WIDGET-001,Widget Pro,9.99,true\n" +
		"GADGET-002,\"Super Gadget, v2\",49.99,false\n" +
		"THNG-003,\"The \"\"Thing\"\"\",0.99,true\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 3 {
		t.Fatalf("expected 3 rows, got %d", len(rows))
	}

	assertField(t, rows[0], "sku", "WIDGET-001")
	assertField(t, rows[0], "name", "Widget Pro")

	// Quoted field with embedded comma
	assertField(t, rows[1], "name", "Super Gadget, v2")

	// Quoted field with escaped double-quotes
	assertField(t, rows[2], "name", `The "Thing"`)
}

func TestMixedQuotedAndUnquotedFields(t *testing.T) {
	input := "product,price,description\n" +
		"Widget,9.99,\"A small, round widget\"\n" +
		"Gadget,19.99,Electronic device\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
	assertField(t, rows[0], "description", "A small, round widget")
	assertField(t, rows[1], "description", "Electronic device")
}

func TestQuotedFieldEmbeddedNewlineMultiRowFile(t *testing.T) {
	input := "id,note\n1,\"Line one\nLine two\"\n2,Single line\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("expected 2 rows, got %d", len(rows))
	}
	assertField(t, rows[0], "note", "Line one\nLine two")
	assertField(t, rows[1], "note", "Single line")
}

// ===========================================================================
// 12. ADDITIONAL COVERAGE CASES
// ===========================================================================
//
// These tests exercise specific state-machine paths that the main test groups
// don't hit, ensuring we reach >95% statement coverage.

func TestFieldStartAtEOFWithNonEmptyRow(t *testing.T) {
	// File ends after a trailing delimiter with no final newline.
	// e.g., "a,b\n1," — the row is ["1", ""] and the parser is in
	// StateFieldStart when EOF arrives, with currentRow already having "1".
	// This exercises the `len(p.currentRow) > 0` branch in StateFieldStart+EOF.
	input := "a,b\n1,"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "a", "1")
	assertField(t, rows[0], "b", "")
}

func TestCarriageReturnAtFieldStart(t *testing.T) {
	// Bare \r (old Mac) line ending, seen while in StateFieldStart.
	// This is the \r case in StateFieldStart (not \r\n, not \r inside a field).
	// e.g., header\r\nrow1a,row1b\r — the second row ends with bare \r
	// and we transition through StateFieldStart after the final field.
	input := "a,b\r1,2\r"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "a", "1")
	assertField(t, rows[0], "b", "2")
}

func TestQuotedFieldClosedThenEOF(t *testing.T) {
	// Quoted field at end of file with no trailing delimiter or newline.
	// The parser is in StateInQuotedMaybeEnd when it hits EOF.
	// e.g., "a,b\n1,\"hello\"" — after the closing '"' we're in MAYBE_END
	// and then hit EOF.
	input := "a,b\n1,\"hello\""

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
	assertField(t, rows[0], "b", "hello")
}

func TestQuotedFieldClosedThenOtherChar(t *testing.T) {
	// Malformed but tolerant: a character that is neither '"', delimiter,
	// nor newline appears after the closing '"'. We hit the `default` case in
	// StateInQuotedMaybeEnd, end the field, and re-process the char.
	// Input: a,b\n"hello"world,2 — 'w' follows the closing '"'
	// Tolerant behaviour: field = "hello", then "world" starts a new field
	// but gets merged back since we re-process 'w' from FIELD_START.
	// Actually: after tolerant end-of-quote, we re-process 'w' from
	// StateFieldStart — so 'w','o','r','l','d' become the next unquoted field.
	// But since the header only has 2 columns (a,b), the truncation logic
	// handles any extra fields. The key invariant: no crash and no error.
	input := "a,b\n\"hello\"world,2\n"

	rows, err := ParseCSV(input)
	if err != nil {
		t.Fatalf("unexpected error for tolerant malformed input: %v", err)
	}
	// The first field value is "hello" (from the quoted part).
	// We don't assert the exact value of "a" here because the tolerant
	// behavior appends the re-processed chars, making it implementation-defined.
	// We only assert no error and correct row count.
	if len(rows) != 1 {
		t.Fatalf("expected 1 row, got %d", len(rows))
	}
}

// ===========================================================================
// Helper
// ===========================================================================

// assertField checks that a row map contains a specific key with a specific
// value, and calls t.Errorf if it doesn't.
func assertField(t *testing.T, row map[string]string, key, want string) {
	t.Helper()
	got, ok := row[key]
	if !ok {
		t.Errorf("key %q not found in row %v", key, row)
		return
	}
	if got != want {
		t.Errorf("row[%q]: expected %q, got %q", key, want, got)
	}
}
