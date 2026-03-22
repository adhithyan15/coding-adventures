package tomlparser

import (
	"testing"
)

// =============================================================================
// TestParseTOMLSimpleKeyValue
// =============================================================================
//
// Verifies that a simple key = value pair can be parsed into an AST.
// The root node should be "document" (the entry rule in toml.grammar).
// Inside the document, there should be an "expression" containing a "keyval".
func TestParseTOMLSimpleKeyValue(t *testing.T) {
	source := `name = "TOML"`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse simple key-value: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}

	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for simple key-value")
	}
}

// =============================================================================
// TestParseTOMLIntegerValue
// =============================================================================
//
// Verifies that integer values can be parsed. Tests decimal, positive,
// negative, and underscore-separated integers.
func TestParseTOMLIntegerValue(t *testing.T) {
	testCases := []string{
		"val = 42",
		"val = +99",
		"val = -17",
		"val = 0",
		"val = 1_000",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse integer %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLHexOctBinInteger
// =============================================================================
//
// Verifies that hex, octal, and binary integers can be parsed.
func TestParseTOMLHexOctBinInteger(t *testing.T) {
	testCases := []string{
		"val = 0xDEADBEEF",
		"val = 0o755",
		"val = 0b11010110",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse prefixed integer %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLFloatValue
// =============================================================================
//
// Verifies that float values can be parsed. Tests decimal, scientific,
// and special float values.
func TestParseTOMLFloatValue(t *testing.T) {
	testCases := []string{
		"val = 3.14",
		"val = -0.01",
		"val = 5e+22",
		"val = 6.626e-34",
		"val = inf",
		"val = -inf",
		"val = nan",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse float %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLBooleanValue
// =============================================================================
//
// Verifies that boolean values (true, false) can be parsed.
func TestParseTOMLBooleanValue(t *testing.T) {
	testCases := []string{
		"flag = true",
		"flag = false",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse boolean %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLStringValues
// =============================================================================
//
// Verifies that all four TOML string types can be parsed as values.
func TestParseTOMLStringValues(t *testing.T) {
	testCases := []string{
		`key = "basic string"`,
		"key = 'literal string'",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse string %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLMultiLineStrings
// =============================================================================
//
// Verifies that multi-line strings (triple-quoted) can be parsed.
func TestParseTOMLMultiLineStrings(t *testing.T) {
	testCases := []string{
		"key = \"\"\"multi\nline\"\"\"",
		"key = '''multi\nline'''",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse multi-line string %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLDateTimeValues
// =============================================================================
//
// Verifies that all four date/time types can be parsed.
func TestParseTOMLDateTimeValues(t *testing.T) {
	testCases := []string{
		"ts = 1979-05-27T07:32:00Z",
		"ts = 1979-05-27T07:32:00",
		"date = 1979-05-27",
		"time = 07:32:00",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse date/time %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLTableHeader
// =============================================================================
//
// Verifies that a table header [name] can be parsed. Table headers switch
// the "current table" — subsequent key-value pairs are added to that table.
func TestParseTOMLTableHeader(t *testing.T) {
	source := "[server]\nhost = \"localhost\"\nport = 8080"
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse table header: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}

	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for table with key-value pairs")
	}
}

// =============================================================================
// TestParseTOMLDottedTableHeader
// =============================================================================
//
// Verifies that dotted table headers [a.b.c] can be parsed. These create
// nested tables.
func TestParseTOMLDottedTableHeader(t *testing.T) {
	source := "[a.b.c]\nval = 1"
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse dotted table header: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLArrayOfTablesHeader
// =============================================================================
//
// Verifies that array-of-tables headers [[name]] can be parsed. Each
// [[name]] header creates a new element in the named array.
func TestParseTOMLArrayOfTablesHeader(t *testing.T) {
	source := "[[products]]\nname = \"Hammer\"\n\n[[products]]\nname = \"Nail\""
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse array-of-tables: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLDottedKey
// =============================================================================
//
// Verifies that dotted keys (a.b.c = val) can be parsed. Dotted keys
// create intermediate tables inline.
func TestParseTOMLDottedKey(t *testing.T) {
	source := `physical.color = "orange"`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse dotted key: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLSimpleArray
// =============================================================================
//
// Verifies that a simple single-line array can be parsed.
func TestParseTOMLSimpleArray(t *testing.T) {
	source := `colors = ["red", "green", "blue"]`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse simple array: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLMultiLineArray
// =============================================================================
//
// Verifies that multi-line arrays (with newlines between elements) can be
// parsed. The array_values rule allows { NEWLINE } at various positions.
func TestParseTOMLMultiLineArray(t *testing.T) {
	source := "colors = [\n  \"red\",\n  \"green\",\n  \"blue\",\n]"
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse multi-line array: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLEmptyArray
// =============================================================================
//
// Verifies that an empty array [] can be parsed.
func TestParseTOMLEmptyArray(t *testing.T) {
	source := `items = []`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse empty array: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLNestedArray
// =============================================================================
//
// Verifies that nested arrays can be parsed. Arrays can contain other arrays.
func TestParseTOMLNestedArray(t *testing.T) {
	source := `matrix = [[1, 2], [3, 4]]`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse nested array: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLInlineTable
// =============================================================================
//
// Verifies that inline tables { key = val, key = val } can be parsed.
func TestParseTOMLInlineTable(t *testing.T) {
	source := `point = { x = 1, y = 2 }`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse inline table: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLEmptyInlineTable
// =============================================================================
//
// Verifies that an empty inline table {} can be parsed.
func TestParseTOMLEmptyInlineTable(t *testing.T) {
	source := `empty = {}`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse empty inline table: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLMultipleKeyValues
// =============================================================================
//
// Verifies that multiple key-value pairs separated by newlines can be parsed.
func TestParseTOMLMultipleKeyValues(t *testing.T) {
	source := "name = \"TOML\"\nversion = 1\nenabled = true"
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse multiple key-values: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}

	if len(program.Children) < 3 {
		t.Errorf("Expected at least 3 children (expressions + newlines), got %d", len(program.Children))
	}
}

// =============================================================================
// TestParseTOMLQuotedKeys
// =============================================================================
//
// Verifies that quoted keys (both basic and literal strings) can be parsed.
func TestParseTOMLQuotedKeys(t *testing.T) {
	testCases := []string{
		`"my key" = 1`,
		"'literal key' = 2",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse quoted key %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLMixedValueArray
// =============================================================================
//
// Verifies that arrays with mixed value types can be parsed. TOML allows
// arrays to contain different types (though the spec recommends homogeneous
// arrays).
func TestParseTOMLMixedValueArray(t *testing.T) {
	source := `mixed = ["hello", 42, true, 1979-05-27]`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse mixed array: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestCreateTOMLParser
// =============================================================================
//
// Verifies that the factory function CreateTOMLParser returns a valid
// GrammarParser instance.
func TestCreateTOMLParser(t *testing.T) {
	source := `key = 42`
	tomlParser, err := CreateTOMLParser(source)
	if err != nil {
		t.Fatalf("Failed to create TOML parser: %v", err)
	}

	if tomlParser == nil {
		t.Fatal("CreateTOMLParser returned nil parser")
	}

	ast, err := tomlParser.Parse()
	if err != nil {
		t.Fatalf("Failed to parse with created parser: %v", err)
	}

	if ast.RuleName != "document" {
		t.Errorf("Expected root rule 'document', got %q", ast.RuleName)
	}
}

// =============================================================================
// TestParseTOMLComplexDocument
// =============================================================================
//
// Verifies parsing of a realistic TOML document that exercises multiple
// features: table headers, array-of-tables, key-value pairs, different
// value types, arrays, dotted keys, and inline tables.
func TestParseTOMLComplexDocument(t *testing.T) {
	source := "# This is a TOML document\ntitle = \"TOML Example\"\n\n[owner]\nname = \"Tom Preston-Werner\"\ndob = 1979-05-27T07:32:00Z\n\n[database]\nenabled = true\nports = [8001, 8001, 8002]\ntemp_targets = { cpu = 79.5, case = 72.0 }\n\n[servers.alpha]\nip = \"10.0.0.1\"\nrole = \"frontend\"\n\n[[products]]\nname = \"Hammer\"\nsku = 738594937\n\n[[products]]\nname = \"Nail\"\nsku = 284758393\ncolor = \"gray\""
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse complex TOML document: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}

	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for complex document")
	}
}

// =============================================================================
// TestParseTOMLEmptyDocument
// =============================================================================
//
// Verifies that an empty TOML document (or one with only comments/blank lines)
// can be parsed. The document rule allows zero expressions.
func TestParseTOMLEmptyDocument(t *testing.T) {
	testCases := []string{
		"",
		"# just a comment",
		"\n\n\n",
		"# comment\n\n# another comment",
	}

	for _, source := range testCases {
		program, err := ParseTOML(source)
		if err != nil {
			t.Fatalf("Failed to parse empty/comment document %q: %v", source, err)
		}

		if program.RuleName != "document" {
			t.Errorf("Expected root rule 'document' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseTOMLTrailingCommaArray
// =============================================================================
//
// Verifies that arrays with trailing commas can be parsed. The array_values
// rule includes an optional trailing [ COMMA ] before the closing bracket.
func TestParseTOMLTrailingCommaArray(t *testing.T) {
	source := "items = [1, 2, 3,]"
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse array with trailing comma: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLSingleElementArray
// =============================================================================
//
// Verifies that a single-element array can be parsed.
func TestParseTOMLSingleElementArray(t *testing.T) {
	source := `items = [42]`
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse single-element array: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLAllValueTypes
// =============================================================================
//
// Verifies that all TOML value types can be parsed in a single document.
func TestParseTOMLAllValueTypes(t *testing.T) {
	source := "str = \"hello\"\nlit = 'world'\nint_val = 42\nflt = 3.14\nbool_val = true\ndate = 1979-05-27\narr = [1, 2]\ninl = { a = 1 }"
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse document with all value types: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseTOMLMultipleTables
// =============================================================================
//
// Verifies that multiple table headers can be parsed in sequence.
func TestParseTOMLMultipleTables(t *testing.T) {
	source := "[a]\nval = 1\n\n[b]\nval = 2\n\n[c]\nval = 3"
	program, err := ParseTOML(source)
	if err != nil {
		t.Fatalf("Failed to parse multiple tables: %v", err)
	}

	if program.RuleName != "document" {
		t.Fatalf("Expected root rule 'document', got %q", program.RuleName)
	}
}
