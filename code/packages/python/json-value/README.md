# JSON Value

Converts json-parser ASTs into typed `JsonValue` objects and native Python
types. This is the bridge between the generic `ASTNode` tree (from
`json-parser`) and meaningful, type-safe JSON data.

## What Is This?

JSON has exactly six value types: object, array, string, number, boolean,
and null. This package provides a Python class for each (`JsonObject`,
`JsonArray`, `JsonString`, `JsonNumber`, `JsonBool`, `JsonNull`), plus
functions to convert between these typed values and native Python types.

## How It Fits in the Stack

```
JSON text  --> json-lexer --> tokens --> json-parser --> AST
                                                          |
                                                     from_ast()
                                                          |
                                                          v
                                                     JsonValue tree
                                                      /        \
                                              to_native()   from_native()
                                                    /            \
                                                   v              v
                                              Native Python   Native Python
                                              (dict, list,    (dict, list,
                                               str, int, ...)  str, int, ...)
```

## Usage

```python
from json_value import parse, parse_native, from_native, to_native

# Parse JSON text into typed JsonValue:
value = parse('{"name": "Alice", "age": 30}')
# value == JsonObject({"name": JsonString("Alice"), "age": JsonNumber(30)})

# Parse JSON text into native Python dict:
data = parse_native('{"name": "Alice", "age": 30}')
# data == {"name": "Alice", "age": 30}

# Convert native Python to JsonValue:
json_val = from_native({"x": 1, "y": [2, 3]})

# Convert JsonValue back to native:
native = to_native(json_val)
# native == {"x": 1, "y": [2, 3]}
```

## Installation

```bash
pip install coding-adventures-json-value
```

## Dependencies

- `coding-adventures-json-parser` -- parses JSON text into ASTs
- `coding-adventures-json-lexer` -- tokenizes JSON text
- `coding-adventures-lexer` -- provides the Token type
- `coding-adventures-parser` -- provides the ASTNode type
- `coding-adventures-grammar-tools` -- parses grammar files
