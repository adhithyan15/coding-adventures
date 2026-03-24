# json-serializer

Converts `JsonValue` types into compact or pretty-printed JSON text. The reverse direction of `json-value`.

## Where it fits

```
JsonValue --> json-serializer --> JSON text
                                    String
```

## Usage

```rust
use coding_adventures_json_value::{JsonValue, JsonNumber};
use coding_adventures_json_serializer::{serialize, serialize_pretty, SerializerConfig};

let val = JsonValue::Object(vec![
    ("name".to_string(), JsonValue::String("Alice".to_string())),
    ("age".to_string(), JsonValue::Number(JsonNumber::Integer(30))),
]);

// Compact: {"name":"Alice","age":30}
let compact = serialize(&val).unwrap();

// Pretty:
// {
//   "name": "Alice",
//   "age": 30
// }
let config = SerializerConfig::default();
let pretty = serialize_pretty(&val, &config).unwrap();
```

## API

- `serialize(value: &JsonValue) -> Result<String, JsonSerializerError>` -- compact JSON
- `serialize_pretty(value: &JsonValue, config: &SerializerConfig) -> Result<String, JsonSerializerError>` -- pretty-printed JSON
- `SerializerConfig` -- indent size, indent char, sort keys, trailing newline

## String escaping

Follows RFC 8259. Escapes `"`, `\`, and control characters (U+0000..U+001F). Forward slash is NOT escaped.

## Dependencies

- `json-value`
