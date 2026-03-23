# json-serializer

Convert JsonValue or native Go types into JSON text, with compact and pretty-printed output modes.

## Overview

This package completes the JSON pipeline by turning typed values back into text. It supports two output modes:

1. **Compact** -- no unnecessary whitespace, smallest output size
2. **Pretty** -- human-readable with configurable indentation

## Where It Fits

```
JSON text
  |-> json-lexer (tokenize)
  |-> json-parser (parse to AST)
  |-> json-value (AST -> typed values)
  |-> json-serializer (THIS PACKAGE: typed values -> JSON text)
```

## Usage

```go
import (
    jsonserializer "github.com/coding-adventures/json-serializer"
    jsonvalue "github.com/coding-adventures/json-value"
)

// Compact serialization from JsonValue
result, err := jsonserializer.Serialize(myJsonValue)

// Pretty-printed with custom config
config := &jsonserializer.SerializerConfig{
    IndentSize: 4,
    IndentChar: ' ',
    SortKeys:   true,
}
result, err := jsonserializer.SerializePretty(myJsonValue, config)

// Convenience: native Go types to compact JSON
result, err := jsonserializer.Stringify(map[string]interface{}{"key": "value"})

// Convenience: native Go types to pretty JSON
result, err := jsonserializer.StringifyPretty(myMap, nil)
```

## API

- `Serialize(value JsonValue) (string, error)` -- compact JSON output
- `SerializePretty(value JsonValue, config *SerializerConfig) (string, error)` -- pretty JSON output
- `Stringify(value interface{}) (string, error)` -- native types to compact JSON
- `StringifyPretty(value interface{}, config *SerializerConfig) (string, error)` -- native types to pretty JSON

## Configuration

- `IndentSize` -- spaces/tabs per level (default: 2)
- `IndentChar` -- `' '` or `'\t'` (default: space)
- `SortKeys` -- sort object keys alphabetically (default: false)
- `TrailingNewline` -- append newline at end (default: false)

## Dependencies

- `json-value` (which depends on json-parser, json-lexer, lexer, parser, grammar-tools)
