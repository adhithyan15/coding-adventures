package jsonserializer

import (
	"math"
	"strings"
	"testing"

	jsonvalue "github.com/coding-adventures/json-value"
)

// ============================================================================
// Tests for Serialize (compact mode)
// ============================================================================

// TestSerializeNull verifies that JsonNull serializes to "null".
func TestSerializeNull(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonNull{})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "null" {
		t.Errorf("expected %q, got %q", "null", result)
	}
}

// TestSerializeNilValue verifies that a nil JsonValue serializes to "null".
func TestSerializeNilValue(t *testing.T) {
	result, err := Serialize(nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "null" {
		t.Errorf("expected %q, got %q", "null", result)
	}
}

// TestSerializeBoolTrue verifies true serialization.
func TestSerializeBoolTrue(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonBool{Value: true})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "true" {
		t.Errorf("expected %q, got %q", "true", result)
	}
}

// TestSerializeBoolFalse verifies false serialization.
func TestSerializeBoolFalse(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonBool{Value: false})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "false" {
		t.Errorf("expected %q, got %q", "false", result)
	}
}

// TestSerializeIntNumber verifies integer number serialization.
func TestSerializeIntNumber(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonNumber{Value: 42, IsInteger: true})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "42" {
		t.Errorf("expected %q, got %q", "42", result)
	}
}

// TestSerializeNegativeNumber verifies negative number serialization.
func TestSerializeNegativeNumber(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonNumber{Value: -5, IsInteger: true})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "-5" {
		t.Errorf("expected %q, got %q", "-5", result)
	}
}

// TestSerializeZero verifies zero serialization.
func TestSerializeZero(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonNumber{Value: 0, IsInteger: true})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "0" {
		t.Errorf("expected %q, got %q", "0", result)
	}
}

// TestSerializeFloatNumber verifies float number serialization.
func TestSerializeFloatNumber(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonNumber{Value: 3.14, IsInteger: false})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "3.14" {
		t.Errorf("expected %q, got %q", "3.14", result)
	}
}

// TestSerializeSimpleString verifies basic string serialization with quoting.
func TestSerializeSimpleString(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: "hello"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"hello"` {
		t.Errorf("expected %q, got %q", `"hello"`, result)
	}
}

// TestSerializeStringWithNewline verifies newline escaping.
func TestSerializeStringWithNewline(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: "a\nb"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"a\nb"` {
		t.Errorf("expected %q, got %q", `"a\nb"`, result)
	}
}

// TestSerializeStringWithQuote verifies quote escaping.
func TestSerializeStringWithQuote(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: `say "hi"`})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := `"say \"hi\""`
	if result != expected {
		t.Errorf("expected %s, got %s", expected, result)
	}
}

// TestSerializeStringWithBackslash verifies backslash escaping.
func TestSerializeStringWithBackslash(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: `a\b`})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := `"a\\b"`
	if result != expected {
		t.Errorf("expected %s, got %s", expected, result)
	}
}

// TestSerializeStringWithTab verifies tab escaping.
func TestSerializeStringWithTab(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: "\t"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"\t"` {
		t.Errorf("expected %q, got %q", `"\t"`, result)
	}
}

// TestSerializeStringWithControlChar verifies control character \u escaping.
func TestSerializeStringWithControlChar(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: "\x00"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"\u0000"` {
		t.Errorf("expected %q, got %q", `"\u0000"`, result)
	}
}

// TestSerializeStringWithMultipleControlChars verifies various control chars.
func TestSerializeStringWithMultipleControlChars(t *testing.T) {
	// Test backspace, form feed, carriage return
	result, err := Serialize(&jsonvalue.JsonString{Value: "\b\f\r"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"\b\f\r"` {
		t.Errorf("expected %q, got %q", `"\b\f\r"`, result)
	}
}

// TestSerializeStringForwardSlashNotEscaped verifies that / is NOT escaped.
func TestSerializeStringForwardSlashNotEscaped(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: "a/b"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"a/b"` {
		t.Errorf("expected %q, got %q", `"a/b"`, result)
	}
}

// TestSerializeEmptyObject verifies empty object serialization.
func TestSerializeEmptyObject(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonObject{Pairs: []jsonvalue.KeyValuePair{}})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "{}" {
		t.Errorf("expected %q, got %q", "{}", result)
	}
}

// TestSerializeSimpleObject verifies single-pair object serialization.
func TestSerializeSimpleObject(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
		},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `{"a":1}` {
		t.Errorf("expected %q, got %q", `{"a":1}`, result)
	}
}

// TestSerializeMultiPairObject verifies multi-pair object serialization.
func TestSerializeMultiPairObject(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
			{Key: "b", Value: &jsonvalue.JsonNumber{Value: 2, IsInteger: true}},
		},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `{"a":1,"b":2}` {
		t.Errorf("expected %q, got %q", `{"a":1,"b":2}`, result)
	}
}

// TestSerializeEmptyArray verifies empty array serialization.
func TestSerializeEmptyArray(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonArray{Elements: []jsonvalue.JsonValue{}})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "[]" {
		t.Errorf("expected %q, got %q", "[]", result)
	}
}

// TestSerializeSimpleArray verifies single-element array serialization.
func TestSerializeSimpleArray(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonArray{
		Elements: []jsonvalue.JsonValue{
			&jsonvalue.JsonNumber{Value: 1, IsInteger: true},
		},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "[1]" {
		t.Errorf("expected %q, got %q", "[1]", result)
	}
}

// TestSerializeMultiElementArray verifies multi-element array serialization.
func TestSerializeMultiElementArray(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonArray{
		Elements: []jsonvalue.JsonValue{
			&jsonvalue.JsonNumber{Value: 1, IsInteger: true},
			&jsonvalue.JsonNumber{Value: 2, IsInteger: true},
			&jsonvalue.JsonNumber{Value: 3, IsInteger: true},
		},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "[1,2,3]" {
		t.Errorf("expected %q, got %q", "[1,2,3]", result)
	}
}

// TestSerializeNestedStructure verifies nested object/array serialization.
func TestSerializeNestedStructure(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "items", Value: &jsonvalue.JsonArray{
				Elements: []jsonvalue.JsonValue{
					&jsonvalue.JsonObject{
						Pairs: []jsonvalue.KeyValuePair{
							{Key: "id", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
						},
					},
				},
			}},
		},
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `{"items":[{"id":1}]}` {
		t.Errorf("expected %q, got %q", `{"items":[{"id":1}]}`, result)
	}
}

// TestSerializeInfinity verifies that Infinity produces an error.
func TestSerializeInfinity(t *testing.T) {
	_, err := Serialize(&jsonvalue.JsonNumber{Value: math.Inf(1), IsInteger: false})
	if err == nil {
		t.Error("expected error for Infinity")
	}
}

// TestSerializeNegativeInfinity verifies that -Infinity produces an error.
func TestSerializeNegativeInfinity(t *testing.T) {
	_, err := Serialize(&jsonvalue.JsonNumber{Value: math.Inf(-1), IsInteger: false})
	if err == nil {
		t.Error("expected error for -Infinity")
	}
}

// TestSerializeNaN verifies that NaN produces an error.
func TestSerializeNaN(t *testing.T) {
	_, err := Serialize(&jsonvalue.JsonNumber{Value: math.NaN(), IsInteger: false})
	if err == nil {
		t.Error("expected error for NaN")
	}
}

// ============================================================================
// Tests for SerializePretty
// ============================================================================

// TestPrettyEmptyObject verifies that empty objects produce "{}".
func TestPrettyEmptyObject(t *testing.T) {
	result, err := SerializePretty(&jsonvalue.JsonObject{Pairs: []jsonvalue.KeyValuePair{}}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "{}" {
		t.Errorf("expected %q, got %q", "{}", result)
	}
}

// TestPrettySimpleObject verifies pretty-printed single-pair object.
func TestPrettySimpleObject(t *testing.T) {
	result, err := SerializePretty(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
		},
	}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := "{\n  \"a\": 1\n}"
	if result != expected {
		t.Errorf("expected %q, got %q", expected, result)
	}
}

// TestPrettyNestedObject verifies indentation increases at each nesting level.
func TestPrettyNestedObject(t *testing.T) {
	result, err := SerializePretty(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "outer", Value: &jsonvalue.JsonObject{
				Pairs: []jsonvalue.KeyValuePair{
					{Key: "inner", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
				},
			}},
		},
	}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := "{\n  \"outer\": {\n    \"inner\": 1\n  }\n}"
	if result != expected {
		t.Errorf("expected:\n%s\ngot:\n%s", expected, result)
	}
}

// TestPrettyArray verifies pretty-printed array.
func TestPrettyArray(t *testing.T) {
	result, err := SerializePretty(&jsonvalue.JsonArray{
		Elements: []jsonvalue.JsonValue{
			&jsonvalue.JsonNumber{Value: 1, IsInteger: true},
			&jsonvalue.JsonNumber{Value: 2, IsInteger: true},
		},
	}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := "[\n  1,\n  2\n]"
	if result != expected {
		t.Errorf("expected %q, got %q", expected, result)
	}
}

// TestPrettyEmptyArray verifies that empty arrays produce "[]".
func TestPrettyEmptyArray(t *testing.T) {
	result, err := SerializePretty(&jsonvalue.JsonArray{Elements: []jsonvalue.JsonValue{}}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "[]" {
		t.Errorf("expected %q, got %q", "[]", result)
	}
}

// TestPrettyCustomIndentSize verifies custom indent size (4 spaces).
func TestPrettyCustomIndentSize(t *testing.T) {
	config := &SerializerConfig{IndentSize: 4, IndentChar: ' '}
	result, err := SerializePretty(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
		},
	}, config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := "{\n    \"a\": 1\n}"
	if result != expected {
		t.Errorf("expected %q, got %q", expected, result)
	}
}

// TestPrettyTabIndent verifies tab indentation.
func TestPrettyTabIndent(t *testing.T) {
	config := &SerializerConfig{IndentSize: 1, IndentChar: '\t'}
	result, err := SerializePretty(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
		},
	}, config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := "{\n\t\"a\": 1\n}"
	if result != expected {
		t.Errorf("expected %q, got %q", expected, result)
	}
}

// TestPrettySortKeys verifies alphabetical key sorting.
func TestPrettySortKeys(t *testing.T) {
	config := &SerializerConfig{IndentSize: 2, IndentChar: ' ', SortKeys: true}
	result, err := SerializePretty(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "c", Value: &jsonvalue.JsonNumber{Value: 3, IsInteger: true}},
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
			{Key: "b", Value: &jsonvalue.JsonNumber{Value: 2, IsInteger: true}},
		},
	}, config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Keys should be sorted: a, b, c
	lines := strings.Split(result, "\n")
	if len(lines) != 5 { // {, a, b, c, }
		t.Fatalf("expected 5 lines, got %d: %q", len(lines), result)
	}
	if !strings.Contains(lines[1], `"a"`) {
		t.Errorf("expected first key 'a', got %q", lines[1])
	}
	if !strings.Contains(lines[2], `"b"`) {
		t.Errorf("expected second key 'b', got %q", lines[2])
	}
	if !strings.Contains(lines[3], `"c"`) {
		t.Errorf("expected third key 'c', got %q", lines[3])
	}
}

// TestPrettyTrailingNewline verifies trailing newline is added when configured.
func TestPrettyTrailingNewline(t *testing.T) {
	config := &SerializerConfig{IndentSize: 2, IndentChar: ' ', TrailingNewline: true}
	result, err := SerializePretty(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
		},
	}, config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.HasSuffix(result, "\n") {
		t.Error("expected trailing newline")
	}
}

// TestPrettyNoTrailingNewline verifies no trailing newline by default.
func TestPrettyNoTrailingNewline(t *testing.T) {
	result, err := SerializePretty(&jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "a", Value: &jsonvalue.JsonNumber{Value: 1, IsInteger: true}},
		},
	}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if strings.HasSuffix(result, "\n") {
		t.Error("unexpected trailing newline")
	}
}

// TestPrettyNilValue verifies that nil produces "null" in pretty mode.
func TestPrettyNilValue(t *testing.T) {
	result, err := SerializePretty(nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "null" {
		t.Errorf("expected %q, got %q", "null", result)
	}
}

// TestPrettyNilTrailingNewline verifies trailing newline with nil value.
func TestPrettyNilTrailingNewline(t *testing.T) {
	config := &SerializerConfig{TrailingNewline: true}
	result, err := SerializePretty(nil, config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "null\n" {
		t.Errorf("expected %q, got %q", "null\n", result)
	}
}

// TestPrettyPrimitives verifies that primitives pretty-print the same as compact.
func TestPrettyPrimitives(t *testing.T) {
	tests := []struct {
		name     string
		value    jsonvalue.JsonValue
		expected string
	}{
		{"null", &jsonvalue.JsonNull{}, "null"},
		{"true", &jsonvalue.JsonBool{Value: true}, "true"},
		{"false", &jsonvalue.JsonBool{Value: false}, "false"},
		{"int", &jsonvalue.JsonNumber{Value: 42, IsInteger: true}, "42"},
		{"float", &jsonvalue.JsonNumber{Value: 3.14, IsInteger: false}, "3.14"},
		{"string", &jsonvalue.JsonString{Value: "hello"}, `"hello"`},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := SerializePretty(tt.value, nil)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if result != tt.expected {
				t.Errorf("expected %q, got %q", tt.expected, result)
			}
		})
	}
}

// ============================================================================
// Tests for Stringify and StringifyPretty
// ============================================================================

// TestStringifyMap verifies map -> compact JSON.
func TestStringifyMap(t *testing.T) {
	result, err := Stringify(map[string]interface{}{"a": 1})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `{"a":1}` {
		t.Errorf("expected %q, got %q", `{"a":1}`, result)
	}
}

// TestStringifySlice verifies slice -> compact JSON.
func TestStringifySlice(t *testing.T) {
	result, err := Stringify([]interface{}{1, 2})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `[1,2]` {
		t.Errorf("expected %q, got %q", `[1,2]`, result)
	}
}

// TestStringifyString verifies string -> compact JSON.
func TestStringifyString(t *testing.T) {
	result, err := Stringify("hello")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"hello"` {
		t.Errorf("expected %q, got %q", `"hello"`, result)
	}
}

// TestStringifyInt verifies int -> compact JSON.
func TestStringifyInt(t *testing.T) {
	result, err := Stringify(42)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "42" {
		t.Errorf("expected %q, got %q", "42", result)
	}
}

// TestStringifyBool verifies bool -> compact JSON.
func TestStringifyBool(t *testing.T) {
	result, err := Stringify(true)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "true" {
		t.Errorf("expected %q, got %q", "true", result)
	}
}

// TestStringifyNil verifies nil -> compact JSON.
func TestStringifyNil(t *testing.T) {
	result, err := Stringify(nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "null" {
		t.Errorf("expected %q, got %q", "null", result)
	}
}

// TestStringifyPrettyMap verifies map -> pretty JSON.
func TestStringifyPrettyMap(t *testing.T) {
	result, err := StringifyPretty(map[string]interface{}{"a": 1}, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := "{\n  \"a\": 1\n}"
	if result != expected {
		t.Errorf("expected %q, got %q", expected, result)
	}
}

// TestStringifyUnsupportedType verifies error for unsupported types.
func TestStringifyUnsupportedType(t *testing.T) {
	_, err := Stringify(struct{}{})
	if err == nil {
		t.Error("expected error for unsupported type")
	}
}

// TestStringifyPrettyUnsupportedType verifies error for unsupported types in pretty mode.
func TestStringifyPrettyUnsupportedType(t *testing.T) {
	_, err := StringifyPretty(struct{}{}, nil)
	if err == nil {
		t.Error("expected error for unsupported type")
	}
}

// ============================================================================
// Full Round-trip Tests (parse + serialize)
// ============================================================================

// TestRoundTripSimpleObject tests parsing then serializing a simple object.
func TestRoundTripSimpleObject(t *testing.T) {
	input := `{"a":1}`
	val, err := jsonvalue.Parse(input)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	output, err := Serialize(val)
	if err != nil {
		t.Fatalf("serialize error: %v", err)
	}
	if output != input {
		t.Errorf("round-trip failed: input=%q, output=%q", input, output)
	}
}

// TestRoundTripComplexStructure tests a complex nested structure.
func TestRoundTripComplexStructure(t *testing.T) {
	input := `{"name":"Alice","age":30,"scores":[95,87,92],"active":true,"address":{"city":"Wonderland"}}`
	val, err := jsonvalue.Parse(input)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	output, err := Serialize(val)
	if err != nil {
		t.Fatalf("serialize error: %v", err)
	}
	if output != input {
		t.Errorf("round-trip failed:\ninput:  %q\noutput: %q", input, output)
	}
}

// TestRoundTripEmptyContainers tests that {} and [] survive round-trip.
func TestRoundTripEmptyContainers(t *testing.T) {
	tests := []string{`{}`, `[]`}
	for _, input := range tests {
		val, err := jsonvalue.Parse(input)
		if err != nil {
			t.Fatalf("parse error for %q: %v", input, err)
		}
		output, err := Serialize(val)
		if err != nil {
			t.Fatalf("serialize error for %q: %v", input, err)
		}
		if output != input {
			t.Errorf("round-trip failed: input=%q, output=%q", input, output)
		}
	}
}

// TestRoundTripAllTypes tests a document with all JSON types.
func TestRoundTripAllTypes(t *testing.T) {
	input := `[1,3.14,"hello",true,false,null,{},[]]`
	val, err := jsonvalue.Parse(input)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	output, err := Serialize(val)
	if err != nil {
		t.Fatalf("serialize error: %v", err)
	}
	if output != input {
		t.Errorf("round-trip failed: input=%q, output=%q", input, output)
	}
}

// TestRoundTripNumberFormats tests various number formats.
func TestRoundTripNumberFormats(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"0", "0"},
		{"42", "42"},
		{"-17", "-17"},
		{"3.14", "3.14"},
	}
	for _, tt := range tests {
		val, err := jsonvalue.Parse(tt.input)
		if err != nil {
			t.Fatalf("parse error for %q: %v", tt.input, err)
		}
		output, err := Serialize(val)
		if err != nil {
			t.Fatalf("serialize error for %q: %v", tt.input, err)
		}
		if output != tt.expected {
			t.Errorf("input=%q: expected %q, got %q", tt.input, tt.expected, output)
		}
	}
}

// TestDefaultConfig verifies the default configuration values.
func TestDefaultConfig(t *testing.T) {
	config := DefaultConfig()
	if config.IndentSize != 2 {
		t.Errorf("expected IndentSize=2, got %d", config.IndentSize)
	}
	if config.IndentChar != ' ' {
		t.Errorf("expected IndentChar=' ', got %c", config.IndentChar)
	}
	if config.SortKeys {
		t.Error("expected SortKeys=false")
	}
	if config.TrailingNewline {
		t.Error("expected TrailingNewline=false")
	}
}

// TestJsonSerializerError verifies the error type's message format.
func TestJsonSerializerError(t *testing.T) {
	err := &JsonSerializerError{Message: "test error"}
	expected := "json serializer error: test error"
	if err.Error() != expected {
		t.Errorf("expected %q, got %q", expected, err.Error())
	}
}

// TestSerializeEmptyString verifies empty string serialization.
func TestSerializeEmptyString(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: ""})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `""` {
		t.Errorf("expected %q, got %q", `""`, result)
	}
}

// TestSerializeLargeNumber verifies large integer serialization.
func TestSerializeLargeNumber(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonNumber{Value: 9999999999999, IsInteger: true})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "9999999999999" {
		t.Errorf("expected %q, got %q", "9999999999999", result)
	}
}

// TestSerializeUnicodeString verifies that non-ASCII characters pass through.
func TestSerializeUnicodeString(t *testing.T) {
	result, err := Serialize(&jsonvalue.JsonString{Value: "cafe\u0301"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Unicode characters above U+001F should NOT be escaped
	if !strings.Contains(result, "\u0301") {
		t.Errorf("expected combining accent to pass through, got %q", result)
	}
}

// TestPrettyComplexNested verifies a complex pretty-printed structure.
func TestPrettyComplexNested(t *testing.T) {
	val := &jsonvalue.JsonObject{
		Pairs: []jsonvalue.KeyValuePair{
			{Key: "users", Value: &jsonvalue.JsonArray{
				Elements: []jsonvalue.JsonValue{
					&jsonvalue.JsonObject{
						Pairs: []jsonvalue.KeyValuePair{
							{Key: "name", Value: &jsonvalue.JsonString{Value: "Alice"}},
							{Key: "age", Value: &jsonvalue.JsonNumber{Value: 30, IsInteger: true}},
						},
					},
				},
			}},
		},
	}
	result, err := SerializePretty(val, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := `{
  "users": [
    {
      "name": "Alice",
      "age": 30
    }
  ]
}`
	if result != expected {
		t.Errorf("expected:\n%s\ngot:\n%s", expected, result)
	}
}

// TestStringifyFloat verifies float -> compact JSON.
func TestStringifyFloat(t *testing.T) {
	result, err := Stringify(3.14)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "3.14" {
		t.Errorf("expected %q, got %q", "3.14", result)
	}
}

// TestStringifyBoolFalse verifies false -> compact JSON.
func TestStringifyBoolFalse(t *testing.T) {
	result, err := Stringify(false)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != "false" {
		t.Errorf("expected %q, got %q", "false", result)
	}
}

// TestSerializeControlCharRange verifies all control characters are escaped.
func TestSerializeControlCharRange(t *testing.T) {
	// Test a control char that's not one of the named escapes (e.g., U+0001)
	result, err := Serialize(&jsonvalue.JsonString{Value: "\x01"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != `"\u0001"` {
		t.Errorf("expected %q, got %q", `"\u0001"`, result)
	}
}

// TestMakeIndent verifies the indent string generation.
func TestMakeIndent(t *testing.T) {
	tests := []struct {
		size     int
		char     rune
		depth    int
		expected string
	}{
		{2, ' ', 0, ""},
		{2, ' ', 1, "  "},
		{2, ' ', 2, "    "},
		{4, ' ', 1, "    "},
		{1, '\t', 1, "\t"},
		{1, '\t', 2, "\t\t"},
	}

	for _, tt := range tests {
		config := &SerializerConfig{IndentSize: tt.size, IndentChar: tt.char}
		result := makeIndent(config, tt.depth)
		if result != tt.expected {
			t.Errorf("makeIndent(size=%d, char=%q, depth=%d): expected %q, got %q",
				tt.size, string(tt.char), tt.depth, tt.expected, result)
		}
	}
}
