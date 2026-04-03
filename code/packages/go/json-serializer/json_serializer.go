// Package jsonserializer converts JsonValue or native Go types into JSON text.
//
// # From Values to Text
//
// This package completes the JSON pipeline. The json-lexer tokenizes text,
// the json-parser builds an AST, the json-value package creates typed values,
// and this package turns those values back into text.
//
// Two output modes are supported:
//
//   - Compact: no unnecessary whitespace, smallest output size.
//     Example: {"name":"Alice","age":30}
//
//   - Pretty: human-readable with configurable indentation.
//     Example:
//     {
//     "name": "Alice",
//     "age": 30
//     }
//
// # String Escaping (RFC 8259)
//
// JSON has strict rules about which characters must be escaped in strings.
// This table summarizes the escaping rules from RFC 8259 Section 7:
//
//	Character          Escape     Why
//	---------          ------     ---
//	" (quotation)      \"         It's the string delimiter
//	\ (backslash)      \\         It's the escape character
//	Backspace (U+08)   \b         Control character
//	Form feed (U+0C)   \f         Control character
//	Newline (U+0A)     \n         Control character
//	Carriage ret (U+0D)\r         Control character
//	Tab (U+09)         \t         Control character
//	U+0000 - U+001F    \uXXXX    All other control characters
//
// Forward slash (/) is NOT escaped. RFC 8259 allows it but does not require
// it, and most JSON implementations do not escape it.
//
// # Configuration
//
// Pretty-printing is controlled by SerializerConfig:
//   - IndentSize: number of indent units per level (default: 2)
//   - IndentChar: character to use (' ' or '\t', default: ' ')
//   - SortKeys: alphabetically sort object keys (default: false)
//   - TrailingNewline: add '\n' at end of output (default: false)
package jsonserializer

import (
	"fmt"
	"math"
	"sort"
	"strconv"
	"strings"

	jsonvalue "github.com/coding-adventures/json-value"
)

// ============================================================================
// Configuration
// ============================================================================

// SerializerConfig controls how pretty-printed JSON is formatted.
//
// Each field has a sensible default (applied when a nil config is passed
// to SerializePretty):
//
//	Field            Default    Meaning
//	-----            -------    -------
//	IndentSize       2          Spaces per indent level
//	IndentChar       ' '        Space or tab
//	SortKeys         false      Preserve insertion order
//	TrailingNewline  false      No extra newline at end
type SerializerConfig struct {
	IndentSize      int
	IndentChar      rune
	SortKeys        bool
	TrailingNewline bool
}

// DefaultConfig returns a SerializerConfig with sensible defaults:
// 2-space indent, no key sorting, no trailing newline.
func DefaultConfig() *SerializerConfig {
	result, _ := StartNew[*SerializerConfig]("json-serializer.DefaultConfig", nil,
		func(op *Operation[*SerializerConfig], rf *ResultFactory[*SerializerConfig]) *OperationResult[*SerializerConfig] {
			return rf.Generate(true, false, &SerializerConfig{
				IndentSize:      2,
				IndentChar:      ' ',
				SortKeys:        false,
				TrailingNewline: false,
			})
		}).GetResult()
	return result
}

// ============================================================================
// Error Type
// ============================================================================

// JsonSerializerError represents an error during JSON serialization.
type JsonSerializerError struct {
	Message string
}

func (e *JsonSerializerError) Error() string {
	return fmt.Sprintf("json serializer error: %s", e.Message)
}

// ============================================================================
// Core API: JsonValue -> Text
// ============================================================================

// Serialize converts a JsonValue to compact JSON text.
//
// Compact mode uses no unnecessary whitespace: no spaces after colons or
// commas, no newlines, no indentation. This produces the smallest possible
// JSON representation, suitable for wire transmission or storage.
//
// Examples:
//
//	Serialize(JsonNull{})                          -> "null"
//	Serialize(JsonBool{true})                      -> "true"
//	Serialize(JsonNumber{42, true})                -> "42"
//	Serialize(JsonString{"hello"})                 -> `"hello"`
//	Serialize(JsonArray{[1, 2]})                   -> "[1,2]"
//	Serialize(JsonObject{[{k:"a", v:1}]})          -> `{"a":1}`
func Serialize(value jsonvalue.JsonValue) (string, error) {
	type serResult struct {
		s   string
		err error
	}
	r, _ := StartNew[serResult]("json-serializer.Serialize", serResult{},
		func(op *Operation[serResult], rf *ResultFactory[serResult]) *OperationResult[serResult] {
			if value == nil {
				return rf.Generate(true, false, serResult{"null", nil})
			}
			s, err := serializeValue(value)
			return rf.Generate(true, false, serResult{s, err})
		}).GetResult()
	return r.s, r.err
}

// SerializePretty converts a JsonValue to pretty-printed JSON text.
//
// Pretty mode adds newlines and indentation to make the output
// human-readable. The formatting is controlled by the config parameter.
// If config is nil, defaults are used (2-space indent).
//
// Example with default config:
//
//	{
//	  "name": "Alice",
//	  "age": 30
//	}
func SerializePretty(value jsonvalue.JsonValue, config *SerializerConfig) (string, error) {
	type serResult struct {
		s   string
		err error
	}
	r, _ := StartNew[serResult]("json-serializer.SerializePretty", serResult{},
		func(op *Operation[serResult], rf *ResultFactory[serResult]) *OperationResult[serResult] {
			if config == nil {
				config = DefaultConfig()
			}
			if value == nil {
				result := "null"
				if config.TrailingNewline {
					result += "\n"
				}
				return rf.Generate(true, false, serResult{result, nil})
			}
			result, err := serializePrettyValue(value, config, 0)
			if err != nil {
				return rf.Generate(true, false, serResult{"", err})
			}
			if config.TrailingNewline {
				result += "\n"
			}
			return rf.Generate(true, false, serResult{result, nil})
		}).GetResult()
	return r.s, r.err
}

// ============================================================================
// Convenience API: Native Types -> Text
// ============================================================================

// Stringify converts native Go types to compact JSON text.
//
// This is a convenience function that combines FromNative and Serialize:
//
//	native value -> JsonValue -> compact JSON text
//
// Example:
//
//	Stringify(map[string]interface{}{"a": 1})  ->  `{"a":1}`
func Stringify(value interface{}) (string, error) {
	type serResult struct {
		s   string
		err error
	}
	r, _ := StartNew[serResult]("json-serializer.Stringify", serResult{},
		func(op *Operation[serResult], rf *ResultFactory[serResult]) *OperationResult[serResult] {
			jv, err := jsonvalue.FromNative(value)
			if err != nil {
				return rf.Generate(true, false, serResult{"", &JsonSerializerError{
					Message: fmt.Sprintf("cannot convert to JsonValue: %s", err.Error()),
				}})
			}
			s, err := serializeValue(jv)
			return rf.Generate(true, false, serResult{s, err})
		}).GetResult()
	return r.s, r.err
}

// StringifyPretty converts native Go types to pretty-printed JSON text.
//
// This is a convenience function that combines FromNative and SerializePretty.
func StringifyPretty(value interface{}, config *SerializerConfig) (string, error) {
	type serResult struct {
		s   string
		err error
	}
	r, _ := StartNew[serResult]("json-serializer.StringifyPretty", serResult{},
		func(op *Operation[serResult], rf *ResultFactory[serResult]) *OperationResult[serResult] {
			if config == nil {
				config = DefaultConfig()
			}
			jv, err := jsonvalue.FromNative(value)
			if err != nil {
				return rf.Generate(true, false, serResult{"", &JsonSerializerError{
					Message: fmt.Sprintf("cannot convert to JsonValue: %s", err.Error()),
				}})
			}
			s, err := serializePrettyValue(jv, config, 0)
			return rf.Generate(true, false, serResult{s, err})
		}).GetResult()
	return r.s, r.err
}

// ============================================================================
// Internal Serialization Logic
// ============================================================================

// serializeValue dispatches on the JsonValue variant and produces compact JSON.
//
// This is the heart of the serializer. It uses a type switch to handle each
// of the six JSON value types, recursing for containers (objects and arrays).
func serializeValue(value jsonvalue.JsonValue) (string, error) {
	switch v := value.(type) {

	case *jsonvalue.JsonNull:
		return "null", nil

	case *jsonvalue.JsonBool:
		if v.Value {
			return "true", nil
		}
		return "false", nil

	case *jsonvalue.JsonNumber:
		return serializeNumber(v)

	case *jsonvalue.JsonString:
		return serializeString(v.Value), nil

	case *jsonvalue.JsonArray:
		return serializeArray(v)

	case *jsonvalue.JsonObject:
		return serializeObject(v)

	default:
		return "", &JsonSerializerError{
			Message: fmt.Sprintf("unknown JsonValue type: %T", value),
		}
	}
}

// serializeNumber converts a JsonNumber to its text representation.
//
// Integer numbers (IsInteger=true) are formatted without a decimal point:
//
//	42.0 (IsInteger=true) -> "42"
//
// Float numbers use Go's strconv.FormatFloat with 'f' or 'g' format to
// avoid unnecessary trailing zeros while preserving precision:
//
//	3.14 -> "3.14"
//	1.0  -> "1"  (trailing zero removed by -1 precision)
//
// Special float values (Infinity, NaN) are rejected because JSON does not
// support them. RFC 8259 Section 6: "Numeric values that cannot be
// represented in the grammar... are not permitted."
func serializeNumber(n *jsonvalue.JsonNumber) (string, error) {
	if math.IsInf(n.Value, 0) {
		return "", &JsonSerializerError{Message: "cannot serialize Infinity"}
	}
	if math.IsNaN(n.Value) {
		return "", &JsonSerializerError{Message: "cannot serialize NaN"}
	}

	if n.IsInteger {
		// Format as integer: no decimal point
		return strconv.FormatInt(int64(n.Value), 10), nil
	}

	// Format as float: use the shortest representation that round-trips
	return strconv.FormatFloat(n.Value, 'f', -1, 64), nil
}

// serializeString wraps a string in double quotes with proper escaping.
//
// This is used for both string values and object keys.
func serializeString(s string) string {
	return `"` + escapeJSONString(s) + `"`
}

// escapeJSONString applies RFC 8259 escaping rules to a string.
//
// The escaping rules form a simple decision tree:
//
//	For each character:
//	  '"'  -> \"      (delimiter must be escaped)
//	  '\\' -> \\      (escape char must be escaped)
//	  '\b' -> \b      (backspace)
//	  '\f' -> \f      (form feed)
//	  '\n' -> \n      (newline)
//	  '\r' -> \r      (carriage return)
//	  '\t' -> \t      (tab)
//	  U+0000..U+001F -> \uXXXX  (other control characters)
//	  everything else -> pass through unchanged
//
// Note: forward slash (/) is NOT escaped. RFC 8259 allows but does not
// require it, and most implementations do not escape it.
func escapeJSONString(s string) string {
	var b strings.Builder
	b.Grow(len(s)) // pre-allocate at least the input length

	for _, r := range s {
		switch r {
		case '"':
			b.WriteString(`\"`)
		case '\\':
			b.WriteString(`\\`)
		case '\b':
			b.WriteString(`\b`)
		case '\f':
			b.WriteString(`\f`)
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		default:
			if r < 0x20 {
				// Control character -- use \uXXXX notation
				b.WriteString(fmt.Sprintf(`\u%04x`, r))
			} else {
				b.WriteRune(r)
			}
		}
	}

	return b.String()
}

// serializeArray converts a JsonArray to compact JSON text.
//
// Empty arrays produce "[]". Non-empty arrays produce comma-separated
// elements with no whitespace: [1,2,3].
func serializeArray(arr *jsonvalue.JsonArray) (string, error) {
	if len(arr.Elements) == 0 {
		return "[]", nil
	}

	parts := make([]string, len(arr.Elements))
	for i, elem := range arr.Elements {
		s, err := serializeValue(elem)
		if err != nil {
			return "", err
		}
		parts[i] = s
	}

	return "[" + strings.Join(parts, ",") + "]", nil
}

// serializeObject converts a JsonObject to compact JSON text.
//
// Empty objects produce "{}". Non-empty objects produce comma-separated
// key:value pairs with no whitespace: {"a":1,"b":2}.
//
// Keys are always strings and are escaped using the same rules as
// string values.
func serializeObject(obj *jsonvalue.JsonObject) (string, error) {
	if len(obj.Pairs) == 0 {
		return "{}", nil
	}

	parts := make([]string, len(obj.Pairs))
	for i, pair := range obj.Pairs {
		valStr, err := serializeValue(pair.Value)
		if err != nil {
			return "", err
		}
		parts[i] = serializeString(pair.Key) + ":" + valStr
	}

	return "{" + strings.Join(parts, ",") + "}", nil
}

// ============================================================================
// Pretty-Print Serialization
// ============================================================================

// serializePrettyValue produces indented JSON text with newlines.
//
// The indentation strategy is straightforward:
//   - Primitives (null, bool, number, string) have no internal structure
//     to indent, so they serialize the same as compact mode.
//   - Arrays place each element on its own line, indented one level deeper.
//   - Objects place each key-value pair on its own line, indented one level deeper.
//
// The indent string is built from the config: IndentChar repeated IndentSize
// times forms one indent level. The current depth determines how many
// indent levels prefix each line.
func serializePrettyValue(value jsonvalue.JsonValue, config *SerializerConfig, depth int) (string, error) {
	switch v := value.(type) {

	case *jsonvalue.JsonNull:
		return "null", nil

	case *jsonvalue.JsonBool:
		if v.Value {
			return "true", nil
		}
		return "false", nil

	case *jsonvalue.JsonNumber:
		return serializeNumber(v)

	case *jsonvalue.JsonString:
		return serializeString(v.Value), nil

	case *jsonvalue.JsonArray:
		return serializePrettyArray(v, config, depth)

	case *jsonvalue.JsonObject:
		return serializePrettyObject(v, config, depth)

	default:
		return "", &JsonSerializerError{
			Message: fmt.Sprintf("unknown JsonValue type: %T", value),
		}
	}
}

// makeIndent creates an indentation string for the given depth.
//
// Example: with IndentSize=2 and IndentChar=' ', depth=3 produces
// "      " (6 spaces = 2 * 3).
func makeIndent(config *SerializerConfig, depth int) string {
	unit := strings.Repeat(string(config.IndentChar), config.IndentSize)
	return strings.Repeat(unit, depth)
}

// serializePrettyArray formats an array with one element per line.
//
// Empty arrays still produce "[]" (no newlines for empty containers).
//
// Non-empty example (depth=0, indent=2):
//
//	[
//	  1,
//	  2,
//	  3
//	]
func serializePrettyArray(arr *jsonvalue.JsonArray, config *SerializerConfig, depth int) (string, error) {
	if len(arr.Elements) == 0 {
		return "[]", nil
	}

	currentIndent := makeIndent(config, depth)
	nextIndent := makeIndent(config, depth+1)

	lines := make([]string, len(arr.Elements))
	for i, elem := range arr.Elements {
		s, err := serializePrettyValue(elem, config, depth+1)
		if err != nil {
			return "", err
		}
		lines[i] = nextIndent + s
	}

	return "[\n" + strings.Join(lines, ",\n") + "\n" + currentIndent + "]", nil
}

// serializePrettyObject formats an object with one pair per line.
//
// Empty objects still produce "{}" (no newlines for empty containers).
//
// Non-empty example (depth=0, indent=2):
//
//	{
//	  "name": "Alice",
//	  "age": 30
//	}
//
// When SortKeys is true, keys are sorted alphabetically. This is useful
// for producing deterministic output regardless of insertion order.
func serializePrettyObject(obj *jsonvalue.JsonObject, config *SerializerConfig, depth int) (string, error) {
	if len(obj.Pairs) == 0 {
		return "{}", nil
	}

	currentIndent := makeIndent(config, depth)
	nextIndent := makeIndent(config, depth+1)

	// Determine key order: original or sorted
	pairs := obj.Pairs
	if config.SortKeys {
		sorted := make([]jsonvalue.KeyValuePair, len(pairs))
		copy(sorted, pairs)
		sort.Slice(sorted, func(i, j int) bool {
			return sorted[i].Key < sorted[j].Key
		})
		pairs = sorted
	}

	lines := make([]string, len(pairs))
	for i, pair := range pairs {
		valStr, err := serializePrettyValue(pair.Value, config, depth+1)
		if err != nil {
			return "", err
		}
		lines[i] = nextIndent + serializeString(pair.Key) + ": " + valStr
	}

	return "{\n" + strings.Join(lines, ",\n") + "\n" + currentIndent + "}", nil
}
