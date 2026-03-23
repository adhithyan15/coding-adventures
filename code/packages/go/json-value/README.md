# json-value

Convert JSON parser ASTs into typed JSON representations, and between typed and native Go types.

## Overview

This package is the bridge between the syntactic world of the json-parser (generic AST nodes) and the semantic world of typed JSON data. It provides:

1. **JsonValue types** -- a Go interface with concrete structs for each JSON type (object, array, string, number, boolean, null)
2. **Native type conversion** -- convert between JsonValue and Go's built-in types (map, slice, string, etc.)

## Where It Fits

```
JSON text
  |-> json-lexer (tokenize)
  |-> json-parser (parse to AST)
  |-> json-value (THIS PACKAGE: AST -> typed values)
  |-> json-serializer (typed values -> JSON text)
```

## Usage

```go
import jsonvalue "github.com/coding-adventures/json-value"

// Parse JSON text into typed values
val, err := jsonvalue.Parse(`{"name": "Alice", "age": 30}`)

// Parse directly into native Go types
native, err := jsonvalue.ParseNative(`{"name": "Alice", "age": 30}`)
// native is map[string]interface{}{"name": "Alice", "age": 30}

// Convert between typed and native
jv, err := jsonvalue.FromNative(map[string]interface{}{"key": "value"})
native = jsonvalue.ToNative(jv)
```

## API

- `Parse(text string) (JsonValue, error)` -- JSON text to JsonValue
- `ParseNative(text string) (interface{}, error)` -- JSON text to native Go types
- `FromAST(node *parser.ASTNode) (JsonValue, error)` -- AST node to JsonValue
- `ToNative(value JsonValue) interface{}` -- JsonValue to native Go types
- `FromNative(value interface{}) (JsonValue, error)` -- native Go types to JsonValue

## Types

- `JsonObject` -- ordered key-value pairs (preserves insertion order)
- `JsonArray` -- ordered sequence of values
- `JsonString` -- string value
- `JsonNumber` -- numeric value with integer/float distinction
- `JsonBool` -- boolean value
- `JsonNull` -- null value

## Dependencies

- `json-parser` (which depends on json-lexer, lexer, parser, grammar-tools)
