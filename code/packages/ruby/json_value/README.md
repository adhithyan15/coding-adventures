# json_value

Typed JSON value representation for Ruby, with conversion between parser ASTs, typed JsonValue objects, and native Ruby types.

## Where It Fits

```
JSON text
  |  (json_parser)
  v
ASTNode tree          -- generic, rule-name-based
  |  (JsonValue.from_ast)
  v
JsonValue tree        -- typed: Object, Array, String, Number, Boolean, Null
  |  (JsonValue.to_native)
  v
Ruby Hash/Array/etc.  -- native types
```

## Usage

```ruby
require "coding_adventures_json_value"

# Parse JSON text into typed JsonValue objects
value = CodingAdventures::JsonValue.parse('{"name": "Alice", "age": 30}')
# => JsonValue::Object with typed children

# Parse JSON text directly into native Ruby types
native = CodingAdventures::JsonValue.parse_native('{"name": "Alice", "age": 30}')
# => {"name" => "Alice", "age" => 30}

# Convert native Ruby types to JsonValue
json_val = CodingAdventures::JsonValue.from_native({"key" => [1, 2, 3]})
# => JsonValue::Object(pairs: {"key" => JsonValue::Array(...)})

# Convert JsonValue back to native types
native = CodingAdventures::JsonValue.to_native(json_val)
# => {"key" => [1, 2, 3]}
```

## JsonValue Types

| JSON Type | Ruby Class | Native Ruby Type |
|-----------|-----------|-----------------|
| `{"key": val}` | `JsonValue::Object` | `Hash` |
| `[1, 2, 3]` | `JsonValue::Array` | `Array` |
| `"hello"` | `JsonValue::String` | `String` |
| `42` | `JsonValue::Number` (integer) | `Integer` |
| `3.14` | `JsonValue::Number` (float) | `Float` |
| `true/false` | `JsonValue::Boolean` | `TrueClass/FalseClass` |
| `null` | `JsonValue::Null` | `nil` |

## Dependencies

- `coding_adventures_json_parser` (and its transitive deps: json_lexer, parser, lexer, grammar_tools)
