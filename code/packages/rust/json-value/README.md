# json-value

Typed representation of JSON data in Rust. Converts the generic AST produced by `json-parser` into a `JsonValue` enum that you can pattern-match on.

## Where it fits

```
JSON text --> json-lexer --> json-parser --> json-value --> application code
  &str        Vec<Token>    GrammarASTNode   JsonValue      your code
```

## JsonValue type

```rust
pub enum JsonValue {
    Object(Vec<(String, JsonValue)>),  // ordered key-value pairs
    Array(Vec<JsonValue>),
    String(String),
    Number(JsonNumber),
    Bool(bool),
    Null,
}

pub enum JsonNumber {
    Integer(i64),
    Float(f64),
}
```

## Usage

```rust
use coding_adventures_json_value::{parse, JsonValue, JsonNumber};

let value = parse(r#"{"name": "Alice", "age": 30}"#).unwrap();

match value {
    JsonValue::Object(pairs) => {
        for (key, val) in &pairs {
            println!("{key}: {val:?}");
        }
    }
    _ => {}
}
```

## API

- `parse(text: &str) -> Result<JsonValue, JsonValueError>` -- parse JSON text into a JsonValue
- `from_ast(node: &GrammarASTNode) -> Result<JsonValue, JsonValueError>` -- convert a parser AST node

## Dependencies

- `json-parser` (which depends on `json-lexer` -> `lexer` -> `grammar-tools`)
