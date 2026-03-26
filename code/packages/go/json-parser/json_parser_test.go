package jsonparser

import (
	"testing"
)

// =============================================================================
// TestParseJSONString
// =============================================================================
//
// Verifies that a standalone JSON string can be parsed into an AST.
// A JSON text can be any value, and a string is the simplest value type.
// The AST root should be a "value" node (the entry rule in json.grammar).
func TestParseJSONString(t *testing.T) {
	source := `"hello"`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse JSON string: %v", err)
	}

	// The root node should be the "value" rule, which is the entry point
	// defined as the first rule in json.grammar.
	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}

	// The value node should have children (the STRING token)
	if len(program.Children) == 0 {
		t.Error("Expected value node to have children")
	}
}

// =============================================================================
// TestParseJSONNumber
// =============================================================================
//
// Verifies that numeric values can be parsed. Tests several forms:
// integer, negative, decimal, and scientific notation.
func TestParseJSONNumber(t *testing.T) {
	testCases := []string{
		"42",
		"-1",
		"3.14",
		"1e10",
		"-0.5e-3",
	}

	for _, source := range testCases {
		program, err := ParseJSON(source)
		if err != nil {
			t.Fatalf("Failed to parse JSON number %q: %v", source, err)
		}

		if program.RuleName != "value" {
			t.Errorf("Expected root rule 'value' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseJSONLiterals
// =============================================================================
//
// Verifies that the three JSON literal values (true, false, null) can be
// parsed. Each is a valid standalone JSON value.
func TestParseJSONLiterals(t *testing.T) {
	testCases := []string{"true", "false", "null"}

	for _, source := range testCases {
		program, err := ParseJSON(source)
		if err != nil {
			t.Fatalf("Failed to parse JSON literal %q: %v", source, err)
		}

		if program.RuleName != "value" {
			t.Errorf("Expected root rule 'value' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseJSONEmptyObject
// =============================================================================
//
// Verifies that an empty object {} can be parsed. This exercises the optional
// pattern [ pair { COMMA pair } ] in the object rule, where the optional
// content is absent.
func TestParseJSONEmptyObject(t *testing.T) {
	source := `{}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse empty object: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}

	// The value should contain an object child
	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for empty object")
	}
}

// =============================================================================
// TestParseJSONEmptyArray
// =============================================================================
//
// Verifies that an empty array [] can be parsed. This exercises the optional
// pattern [ value { COMMA value } ] in the array rule.
func TestParseJSONEmptyArray(t *testing.T) {
	source := `[]`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse empty array: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONSimpleObject
// =============================================================================
//
// Verifies that a simple object with one key-value pair can be parsed.
// This exercises the object and pair rules:
//   object = LBRACE [ pair { COMMA pair } ] RBRACE ;
//   pair   = STRING COLON value ;
func TestParseJSONSimpleObject(t *testing.T) {
	source := `{"name": "Alice"}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse simple object: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}

	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for simple object")
	}
}

// =============================================================================
// TestParseJSONMultiKeyObject
// =============================================================================
//
// Verifies that an object with multiple key-value pairs can be parsed.
// This exercises the repetition { COMMA pair } in the object rule.
func TestParseJSONMultiKeyObject(t *testing.T) {
	source := `{"name": "Bob", "age": 25, "active": true}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse multi-key object: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONSimpleArray
// =============================================================================
//
// Verifies that a simple array with multiple elements can be parsed.
// This exercises the array rule:
//   array = LBRACKET [ value { COMMA value } ] RBRACKET ;
func TestParseJSONSimpleArray(t *testing.T) {
	source := `[1, 2, 3]`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse simple array: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONMixedArray
// =============================================================================
//
// Verifies that an array with mixed value types can be parsed. JSON arrays
// can contain any mix of strings, numbers, booleans, null, objects, and
// other arrays.
func TestParseJSONMixedArray(t *testing.T) {
	source := `["hello", 42, true, false, null]`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse mixed array: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONNestedObject
// =============================================================================
//
// Verifies that nested objects can be parsed. This exercises the recursive
// nature of JSON: a value in a pair can itself be an object.
//   {"person": {"name": "Alice", "age": 30}}
func TestParseJSONNestedObject(t *testing.T) {
	source := `{"person": {"name": "Alice", "age": 30}}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse nested object: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONNestedArray
// =============================================================================
//
// Verifies that nested arrays can be parsed. Arrays containing arrays
// exercise the recursive nature of the grammar.
//   [[1, 2], [3, 4], [5]]
func TestParseJSONNestedArray(t *testing.T) {
	source := `[[1, 2], [3, 4], [5]]`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse nested array: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONObjectWithArray
// =============================================================================
//
// Verifies that objects containing arrays can be parsed. This is a common
// JSON pattern for representing collections:
//   {"items": [1, 2, 3]}
func TestParseJSONObjectWithArray(t *testing.T) {
	source := `{"items": [1, 2, 3]}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse object with array: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONArrayOfObjects
// =============================================================================
//
// Verifies that arrays of objects can be parsed. This is the most common
// JSON pattern for representing lists of records:
//   [{"name": "Alice"}, {"name": "Bob"}]
func TestParseJSONArrayOfObjects(t *testing.T) {
	source := `[{"name": "Alice"}, {"name": "Bob"}]`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse array of objects: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONDeeplyNested
// =============================================================================
//
// Verifies that deeply nested JSON can be parsed. The recursive grammar
// should handle arbitrary nesting depth (limited only by stack space).
//   {"a": {"b": {"c": {"d": 42}}}}
func TestParseJSONDeeplyNested(t *testing.T) {
	source := `{"a": {"b": {"c": {"d": 42}}}}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse deeply nested JSON: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONMultilineFormatted
// =============================================================================
//
// Verifies that pretty-printed (multi-line) JSON can be parsed. The parser
// should handle JSON with indentation and newlines between tokens, since
// whitespace is insignificant in JSON.
func TestParseJSONMultilineFormatted(t *testing.T) {
	source := `{
  "name": "Alice",
  "hobbies": [
    "reading",
    "coding"
  ],
  "active": true
}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse multi-line JSON: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestCreateJSONParser
// =============================================================================
//
// Verifies that the factory function CreateJSONParser returns a valid
// GrammarParser instance. This tests the two-step API (create parser, then
// call Parse) as opposed to the one-shot ParseJSON convenience function.
func TestCreateJSONParser(t *testing.T) {
	source := `42`
	jsonParser, err := CreateJSONParser(source)
	if err != nil {
		t.Fatalf("Failed to create JSON parser: %v", err)
	}

	// The parser should not be nil
	if jsonParser == nil {
		t.Fatal("CreateJSONParser returned nil parser")
	}

	// Parse using the created parser instance
	ast, err := jsonParser.Parse()
	if err != nil {
		t.Fatalf("Failed to parse with created parser: %v", err)
	}

	// The root node should be "value"
	if ast.RuleName != "value" {
		t.Errorf("Expected root rule 'value', got %q", ast.RuleName)
	}
}

// =============================================================================
// TestParseJSONComplexDocument
// =============================================================================
//
// Verifies parsing of a realistic JSON document that exercises all features:
// nested objects, arrays, mixed types, and various value kinds. This is a
// representative test of real-world JSON data.
func TestParseJSONComplexDocument(t *testing.T) {
	source := `{
  "users": [
    {
      "id": 1,
      "name": "Alice",
      "email": "alice@example.com",
      "scores": [95, 87, 92],
      "active": true
    },
    {
      "id": 2,
      "name": "Bob",
      "email": null,
      "scores": [],
      "active": false
    }
  ],
  "total": 2,
  "metadata": {
    "page": 1,
    "limit": 10
  }
}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse complex JSON document: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}

	// A complex document should produce a non-trivial AST
	if len(program.Children) == 0 {
		t.Error("Expected non-empty AST for complex document")
	}
}

// =============================================================================
// TestParseJSONNegativeNumbers
// =============================================================================
//
// Verifies that negative numbers parse correctly in various contexts.
// The minus sign is part of the NUMBER token, not a separate operator.
func TestParseJSONNegativeNumbers(t *testing.T) {
	testCases := []string{
		`-42`,
		`[-1, -2, -3]`,
		`{"temp": -10.5}`,
	}

	for _, source := range testCases {
		program, err := ParseJSON(source)
		if err != nil {
			t.Fatalf("Failed to parse negative number in %q: %v", source, err)
		}

		if program.RuleName != "value" {
			t.Errorf("Expected root rule 'value' for %q, got %q", source, program.RuleName)
		}
	}
}

// =============================================================================
// TestParseJSONSingleElementArray
// =============================================================================
//
// Verifies that an array with a single element parses correctly. This tests
// the optional pattern where the repetition { COMMA value } matches zero times.
func TestParseJSONSingleElementArray(t *testing.T) {
	source := `[42]`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse single-element array: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONSinglePairObject
// =============================================================================
//
// Verifies that an object with a single key-value pair parses correctly.
func TestParseJSONSinglePairObject(t *testing.T) {
	source := `{"key": "value"}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse single-pair object: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

// =============================================================================
// TestParseJSONAllValueTypes
// =============================================================================
//
// Verifies that all seven JSON value types can appear in a single document.
// This is a comprehensive test that exercises every alternative in the
// value rule: object | array | STRING | NUMBER | TRUE | FALSE | NULL
func TestParseJSONAllValueTypes(t *testing.T) {
	source := `{
  "string": "hello",
  "number": 42,
  "negative": -3.14,
  "bool_true": true,
  "bool_false": false,
  "nothing": null,
  "object": {"nested": "yes"},
  "array": [1, "two", true]
}`
	program, err := ParseJSON(source)
	if err != nil {
		t.Fatalf("Failed to parse document with all value types: %v", err)
	}

	if program.RuleName != "value" {
		t.Fatalf("Expected root rule 'value', got %q", program.RuleName)
	}
}

