// Package jsonvalue converts JSON parser ASTs into typed JSON representations.
//
// # The Bridge Between Syntax and Semantics
//
// The json-parser package produces a generic AST (Abstract Syntax Tree) that
// faithfully represents the syntactic structure of JSON text. But syntax alone
// is not enough -- we need semantics. This package provides the bridge.
//
// Consider the JSON text: {"name": "Alice", "age": 30}
//
// The parser sees it as a tree of grammar rules and tokens:
//
//	value
//	  object
//	    LBRACE("{")
//	    pair
//	      STRING("name")
//	      COLON(":")
//	      value
//	        STRING("Alice")
//	    COMMA(",")
//	    pair
//	      STRING("age")
//	      COLON(":")
//	      value
//	        NUMBER("30")
//	    RBRACE("}")
//
// This package transforms that tree into typed values:
//
//	JsonObject{
//	  Pairs: [
//	    {Key: "name", Value: JsonString{Value: "Alice"}},
//	    {Key: "age",  Value: JsonNumber{Value: 30, IsInteger: true}},
//	  ]
//	}
//
// # Two Representations
//
// This package offers two ways to work with JSON data:
//
// 1. JsonValue (typed) -- a Go interface with concrete struct types for each
//    JSON type. Use this when you need type safety, pattern matching via type
//    switches, or custom traversal.
//
// 2. Native Go types (dynamic) -- map[string]interface{}, []interface{},
//    string, float64/int, bool, nil. Use this when you just want to read
//    JSON data without caring about the type system.
//
// # JSON Has Exactly Six Types
//
// RFC 8259 defines six value types. Our JsonValue interface mirrors them:
//
//	JSON Type   Go Representation           Example
//	---------   -----------------           -------
//	object      JsonObject                  {"key": "val"}
//	array       JsonArray                   [1, 2, 3]
//	string      JsonString                  "hello"
//	number      JsonNumber                  42, 3.14
//	boolean     JsonBool                    true, false
//	null        JsonNull                    null
//
// # Integer vs Float Numbers
//
// JSON itself does not distinguish integers from floats -- they're all
// "numbers." But practically, 42 and 3.14 are different. We preserve this
// distinction using the IsInteger flag on JsonNumber:
//
//	42    -> JsonNumber{Value: 42.0, IsInteger: true}
//	3.14  -> JsonNumber{Value: 3.14, IsInteger: false}
//	1e10  -> JsonNumber{Value: 1e10, IsInteger: false}
//
// The rule is simple: if the original text contains a decimal point or
// exponent, it's a float. Otherwise, it's an integer.
package jsonvalue

import (
	"fmt"
	"strconv"
	"strings"

	jsonparser "github.com/adhithyan15/coding-adventures/code/packages/go/json-parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// JsonValue Interface
// ============================================================================

// JsonValue is the interface implemented by all JSON value types.
//
// The interface uses a marker method pattern: jsonValue() is unexported and
// has no behavior. Its sole purpose is to restrict the set of types that can
// satisfy the interface to the six JSON value types defined in this package.
//
// This is a common Go pattern for creating "sealed" interfaces -- types
// outside this package cannot implement JsonValue because they cannot
// define an unexported method from another package.
type JsonValue interface {
	jsonValue() // marker method -- restricts implementors to this package
}

// ============================================================================
// Concrete Types
// ============================================================================

// KeyValuePair represents a single key-value pair in a JSON object.
//
// We use a slice of KeyValuePair rather than a map to preserve insertion
// order. While RFC 8259 says JSON objects are "unordered," preserving order
// matters for:
//   - Human readability (keys appear in the author's intended order)
//   - Round-trip fidelity (parse then serialize produces the same key order)
//   - Deterministic output (same input always produces same output)
type KeyValuePair struct {
	Key   string
	Value JsonValue
}

// JsonObject represents a JSON object: an ordered collection of key-value pairs.
//
// Example JSON: {"name": "Alice", "age": 30}
// Represented as: JsonObject{Pairs: [{Key: "name", Value: JsonString{"Alice"}},
//
//	{Key: "age", Value: JsonNumber{30, true}}]}
type JsonObject struct {
	Pairs []KeyValuePair
}

func (JsonObject) jsonValue() {}

// JsonArray represents a JSON array: an ordered sequence of values.
//
// Example JSON: [1, "two", true]
// Represented as: JsonArray{Elements: [JsonNumber{1, true}, JsonString{"two"}, JsonBool{true}]}
type JsonArray struct {
	Elements []JsonValue
}

func (JsonArray) jsonValue() {}

// JsonString represents a JSON string value.
//
// The Value field contains the unescaped string content. The lexer has
// already processed escape sequences (\n -> newline, \" -> quote, etc.)
// and stripped the surrounding quotes.
type JsonString struct {
	Value string
}

func (JsonString) jsonValue() {}

// JsonNumber represents a JSON number value.
//
// Value holds the numeric value as a float64 (Go's standard numeric type
// for JSON, matching encoding/json behavior). IsInteger indicates whether
// the original JSON text represented an integer (no decimal point or exponent).
//
// Examples:
//
//	42    -> JsonNumber{Value: 42.0, IsInteger: true}
//	-5    -> JsonNumber{Value: -5.0, IsInteger: true}
//	3.14  -> JsonNumber{Value: 3.14, IsInteger: false}
//	1e10  -> JsonNumber{Value: 1e10, IsInteger: false}
type JsonNumber struct {
	Value     float64
	IsInteger bool
}

func (JsonNumber) jsonValue() {}

// JsonBool represents a JSON boolean value (true or false).
type JsonBool struct {
	Value bool
}

func (JsonBool) jsonValue() {}

// JsonNull represents the JSON null value.
//
// JSON null maps to Go's nil, but as a JsonValue we need a concrete type
// to hold it. JsonNull is a zero-size struct -- it carries no data, just
// the information that the value is null.
type JsonNull struct{}

func (JsonNull) jsonValue() {}

// ============================================================================
// Error Type
// ============================================================================

// JsonValueError represents an error during JSON value conversion.
type JsonValueError struct {
	Message string
}

func (e *JsonValueError) Error() string {
	return fmt.Sprintf("json value error: %s", e.Message)
}

// ============================================================================
// AST -> JsonValue Conversion
// ============================================================================

// FromAST converts a json-parser AST node into a JsonValue.
//
// This is a recursive tree walk that dispatches on the node's rule name
// and token types. The algorithm mirrors the JSON grammar:
//
//	value  -> unwrap to find the meaningful child
//	object -> collect pairs into JsonObject
//	pair   -> extract key (STRING) and value (recursive)
//	array  -> collect elements into JsonArray
//
// For leaf tokens:
//
//	STRING -> JsonString (already unescaped by lexer)
//	NUMBER -> JsonNumber (int if no '.' or 'e/E', float otherwise)
//	TRUE   -> JsonBool(true)
//	FALSE  -> JsonBool(false)
//	NULL   -> JsonNull
//
// Returns a JsonValueError if the AST contains unexpected structure.
func FromAST(node *parser.ASTNode) (JsonValue, error) {
	if node == nil {
		return nil, &JsonValueError{Message: "nil AST node"}
	}

	// The grammar parser produces ASTNodes with rule names. The rule name
	// tells us what kind of JSON construct we're looking at.
	switch node.RuleName {

	case "value":
		// The "value" rule is a wrapper: it contains exactly one meaningful
		// child -- either an ASTNode (object or array) or a Token (string,
		// number, true, false, null). We find that child and recurse.
		return fromASTValue(node)

	case "object":
		// An object node contains LBRACE, zero or more pair nodes, and RBRACE.
		// We extract just the pair nodes.
		return fromASTObject(node)

	case "array":
		// An array node contains LBRACKET, zero or more value nodes, and RBRACKET.
		return fromASTArray(node)

	case "pair":
		// A pair node appears as a child of an object -- we shouldn't
		// encounter it at the top level, but handle it gracefully.
		key, val, err := fromASTPair(node)
		if err != nil {
			return nil, err
		}
		return &JsonObject{Pairs: []KeyValuePair{{Key: key, Value: val}}}, nil

	default:
		// Check if this is a leaf node wrapping a token
		if node.IsLeaf() {
			return fromASTToken(node.Token())
		}
		return nil, &JsonValueError{Message: fmt.Sprintf("unexpected rule: %s", node.RuleName)}
	}
}

// fromASTValue handles the "value" rule, which wraps exactly one meaningful child.
//
// The value rule in JSON grammar is:
//
//	value = object | array | STRING | NUMBER | TRUE | FALSE | NULL
//
// So a value node's children will include the meaningful child plus possibly
// structural tokens. We find the first child that carries semantic meaning.
func fromASTValue(node *parser.ASTNode) (JsonValue, error) {
	for _, child := range node.Children {
		switch c := child.(type) {
		case *parser.ASTNode:
			// Child is an ASTNode -- it's either "object" or "array"
			return FromAST(c)

		case lexer.Token:
			// Child is a Token -- check if it's a meaningful value token
			val, err := fromASTToken(&c)
			if err != nil {
				continue // skip structural tokens (LBRACE, COMMA, etc.)
			}
			return val, nil
		}
	}
	return nil, &JsonValueError{Message: "value node has no meaningful children"}
}

// fromASTObject handles the "object" rule.
//
// An object in JSON grammar is:
//
//	object = LBRACE [ pair { COMMA pair } ] RBRACE
//
// We iterate through children, collecting only the "pair" ASTNodes.
// The LBRACE, COMMA, and RBRACE tokens are structural and ignored.
func fromASTObject(node *parser.ASTNode) (JsonValue, error) {
	pairs := []KeyValuePair{}

	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok && childNode.RuleName == "pair" {
			key, val, err := fromASTPair(childNode)
			if err != nil {
				return nil, err
			}
			pairs = append(pairs, KeyValuePair{Key: key, Value: val})
		}
	}

	return &JsonObject{Pairs: pairs}, nil
}

// fromASTPair handles the "pair" rule.
//
// A pair in JSON grammar is:
//
//	pair = STRING COLON value
//
// We find the STRING token (the key) and the "value" ASTNode (the value),
// then recurse on the value.
func fromASTPair(node *parser.ASTNode) (string, JsonValue, error) {
	var key string
	var val JsonValue
	keyFound := false

	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			if c.TypeName == "STRING" && !keyFound {
				key = c.Value
				keyFound = true
			}
		case *parser.ASTNode:
			if c.RuleName == "value" {
				var err error
				val, err = FromAST(c)
				if err != nil {
					return "", nil, err
				}
			}
		}
	}

	if !keyFound {
		return "", nil, &JsonValueError{Message: "pair has no STRING key"}
	}
	if val == nil {
		return "", nil, &JsonValueError{Message: "pair has no value"}
	}

	return key, val, nil
}

// fromASTArray handles the "array" rule.
//
// An array in JSON grammar is:
//
//	array = LBRACKET [ value { COMMA value } ] RBRACKET
//
// We iterate through children, collecting "value" ASTNodes and recursing.
// The LBRACKET, COMMA, and RBRACKET tokens are structural and ignored.
func fromASTArray(node *parser.ASTNode) (JsonValue, error) {
	elements := []JsonValue{}

	for _, child := range node.Children {
		switch c := child.(type) {
		case *parser.ASTNode:
			if c.RuleName == "value" {
				elem, err := FromAST(c)
				if err != nil {
					return nil, err
				}
				elements = append(elements, elem)
			}
		case lexer.Token:
			// Handle edge case: array elements might be direct Token children
			val, err := fromASTToken(&c)
			if err == nil {
				elements = append(elements, val)
			}
		}
	}

	return &JsonArray{Elements: elements}, nil
}

// fromASTToken converts a single token into a JsonValue.
//
// Token type mapping:
//
//	TypeName     JsonValue               Notes
//	--------     ---------               -----
//	STRING       JsonString              Value already unescaped by lexer
//	NUMBER       JsonNumber              Detect int vs float from text
//	TRUE         JsonBool(true)          Literal keyword
//	FALSE        JsonBool(false)         Literal keyword
//	NULL         JsonNull                Literal keyword
//
// Returns an error for structural tokens (LBRACE, COMMA, etc.) that
// don't represent values.
func fromASTToken(tok *lexer.Token) (JsonValue, error) {
	switch tok.TypeName {
	case "STRING":
		return &JsonString{Value: tok.Value}, nil

	case "NUMBER":
		return parseNumber(tok.Value)

	case "TRUE":
		return &JsonBool{Value: true}, nil

	case "FALSE":
		return &JsonBool{Value: false}, nil

	case "NULL":
		return &JsonNull{}, nil

	default:
		return nil, &JsonValueError{
			Message: fmt.Sprintf("not a value token: %s", tok.TypeName),
		}
	}
}

// parseNumber converts a JSON number string into a JsonNumber.
//
// The integer vs float distinction is based on the original text:
//   - "42"   -> integer (no decimal point, no exponent)
//   - "-17"  -> integer (negative, but still no decimal/exponent)
//   - "3.14" -> float  (has decimal point)
//   - "1e10" -> float  (has exponent)
//   - "2.5e3"-> float  (has both)
//
// This matches the behavior of Python's json.loads (42 -> int, 3.14 -> float)
// and Ruby's JSON.parse (42 -> Integer, 3.14 -> Float).
func parseNumber(text string) (JsonValue, error) {
	isFloat := strings.ContainsAny(text, ".eE")

	val, err := strconv.ParseFloat(text, 64)
	if err != nil {
		return nil, &JsonValueError{
			Message: fmt.Sprintf("invalid number: %s", text),
		}
	}

	return &JsonNumber{Value: val, IsInteger: !isFloat}, nil
}

// ============================================================================
// JsonValue -> Native Go Types
// ============================================================================

// ToNative converts a JsonValue into native Go types.
//
// The mapping is:
//
//	JsonValue Type    Go Native Type           Example
//	--------------    --------------           -------
//	JsonObject        map[string]interface{}   {"name": "Alice"}
//	JsonArray         []interface{}            [1, 2, 3]
//	JsonString        string                   "hello"
//	JsonNumber(int)   float64                  42.0 (or int if IsInteger)
//	JsonNumber(float) float64                  3.14
//	JsonBool          bool                     true
//	JsonNull          nil                      nil
//
// For JsonNumber with IsInteger=true, we return an int (not float64) to
// preserve the integer semantics. This means 42 round-trips as an int,
// not as 42.0.
//
// The conversion is recursive -- nested JsonValues are also converted.
func ToNative(value JsonValue) interface{} {
	if value == nil {
		return nil
	}

	switch v := value.(type) {
	case *JsonObject:
		result := make(map[string]interface{})
		for _, pair := range v.Pairs {
			result[pair.Key] = ToNative(pair.Value)
		}
		return result

	case *JsonArray:
		result := make([]interface{}, len(v.Elements))
		for i, elem := range v.Elements {
			result[i] = ToNative(elem)
		}
		return result

	case *JsonString:
		return v.Value

	case *JsonNumber:
		if v.IsInteger {
			return int(v.Value)
		}
		return v.Value

	case *JsonBool:
		return v.Value

	case *JsonNull:
		return nil

	default:
		return nil
	}
}

// ============================================================================
// Native Go Types -> JsonValue
// ============================================================================

// FromNative converts native Go types into a JsonValue.
//
// The mapping is:
//
//	Go Type                    JsonValue Type
//	-------                    --------------
//	map[string]interface{}     JsonObject
//	[]interface{}              JsonArray
//	string                     JsonString
//	int, int8..int64           JsonNumber (IsInteger=true)
//	uint, uint8..uint64        JsonNumber (IsInteger=true)
//	float32, float64           JsonNumber (IsInteger=false)
//	bool                       JsonBool
//	nil                        JsonNull
//
// Returns a JsonValueError if the value contains types that have no JSON
// representation (functions, channels, structs, etc.).
//
// Note: map keys must be strings. Non-string map keys produce an error.
func FromNative(value interface{}) (JsonValue, error) {
	if value == nil {
		return &JsonNull{}, nil
	}

	switch v := value.(type) {
	case map[string]interface{}:
		pairs := make([]KeyValuePair, 0, len(v))
		for key, val := range v {
			jv, err := FromNative(val)
			if err != nil {
				return nil, err
			}
			pairs = append(pairs, KeyValuePair{Key: key, Value: jv})
		}
		return &JsonObject{Pairs: pairs}, nil

	case []interface{}:
		elements := make([]JsonValue, len(v))
		for i, elem := range v {
			jv, err := FromNative(elem)
			if err != nil {
				return nil, err
			}
			elements[i] = jv
		}
		return &JsonArray{Elements: elements}, nil

	case string:
		return &JsonString{Value: v}, nil

	case int:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case int8:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case int16:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case int32:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case int64:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case uint:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case uint8:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case uint16:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case uint32:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil
	case uint64:
		return &JsonNumber{Value: float64(v), IsInteger: true}, nil

	case float32:
		return &JsonNumber{Value: float64(v), IsInteger: false}, nil
	case float64:
		return &JsonNumber{Value: v, IsInteger: false}, nil

	case bool:
		return &JsonBool{Value: v}, nil

	case JsonValue:
		// Already a JsonValue -- return as-is
		return v, nil

	default:
		return nil, &JsonValueError{
			Message: fmt.Sprintf("unsupported type: %T", value),
		}
	}
}

// ============================================================================
// Convenience Functions
// ============================================================================

// Parse parses JSON text into a JsonValue.
//
// This is the full pipeline: text -> lexer -> parser -> AST -> JsonValue.
// It combines the json-parser and FromAST into a single call.
//
// Example:
//
//	val, err := Parse(`{"name": "Alice", "age": 30}`)
//	// val is *JsonObject with two pairs
func Parse(text string) (JsonValue, error) {
	ast, err := jsonparser.ParseJSON(text)
	if err != nil {
		return nil, &JsonValueError{
			Message: fmt.Sprintf("parse error: %s", err.Error()),
		}
	}
	return FromAST(ast)
}

// ParseNative parses JSON text directly into native Go types.
//
// This is the most common use case: "give me a map from this JSON string."
// Equivalent to ToNative(Parse(text)).
//
// Example:
//
//	result, err := ParseNative(`{"name": "Alice", "age": 30}`)
//	// result is map[string]interface{}{"name": "Alice", "age": 30}
func ParseNative(text string) (interface{}, error) {
	val, err := Parse(text)
	if err != nil {
		return nil, err
	}
	return ToNative(val), nil
}
