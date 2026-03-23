# json-value

Typed JSON value representation for TypeScript. Converts json-parser ASTs into a discriminated union type (`JsonValue`) and provides conversion to/from native JavaScript types.

## Where It Fits

```
JSON text --> json-lexer --> json-parser --> AST --> json-value --> JsonValue / native
```

This package is the bridge between generic AST nodes (produced by `json-parser`) and meaningful, typed data that application code can work with.

## Installation

```bash
npm install coding-adventures-json-value
```

## Usage

### Parse JSON text to typed values

```typescript
import { parse, parseNative } from "coding-adventures-json-value";

// Get a typed JsonValue tree
const value = parse('{"name": "Alice", "age": 30}');
// value.type === 'object'
// value.pairs.get('name') === { type: 'string', value: 'Alice' }

// Or get plain JavaScript types directly
const native = parseNative('{"name": "Alice", "age": 30}');
// { name: "Alice", age: 30 }
```

### Convert between JsonValue and native types

```typescript
import { fromNative, toNative, jsonObject, jsonString, jsonNumber } from "coding-adventures-json-value";

// Native to JsonValue
const jv = fromNative({ name: "Alice", scores: [95, 87, 92] });

// JsonValue to native
const native = toNative(jv);
// { name: "Alice", scores: [95, 87, 92] }
```

### Factory functions

```typescript
import { jsonObject, jsonArray, jsonString, jsonNumber, jsonBool, jsonNull } from "coding-adventures-json-value";

const value = jsonObject(new Map([
  ["name", jsonString("Alice")],
  ["age", jsonNumber(30)],
  ["active", jsonBool(true)],
  ["scores", jsonArray([jsonNumber(95), jsonNumber(87)])],
  ["address", jsonNull()],
]));
```

## API

| Function | Description |
|----------|-------------|
| `parse(text)` | Parse JSON text into a JsonValue tree |
| `parseNative(text)` | Parse JSON text into native JS types |
| `fromAST(node)` | Convert a json-parser ASTNode to JsonValue |
| `toNative(value)` | Convert JsonValue to native JS types |
| `fromNative(value)` | Convert native JS types to JsonValue |
| `jsonObject(pairs?)` | Create a JSON object value |
| `jsonArray(elements?)` | Create a JSON array value |
| `jsonString(value)` | Create a JSON string value |
| `jsonNumber(value)` | Create a JSON number value |
| `jsonBool(value)` | Create a JSON boolean value |
| `jsonNull()` | Create a JSON null value |

## Dependencies

- `@coding-adventures/json-parser` -- for parsing JSON text into ASTs

## License

MIT
