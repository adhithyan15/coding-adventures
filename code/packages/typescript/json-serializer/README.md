# json-serializer

JSON serializer for TypeScript. Converts `JsonValue` trees or native JavaScript types to compact or pretty-printed JSON text.

## Where It Fits

```
JsonValue tree  -->  json-serializer  -->  JSON text
native types    -->  json-serializer  -->  JSON text
```

This package is the output stage of the JSON pipeline. It takes typed `JsonValue` trees (from `json-value`) or plain JavaScript objects/arrays and produces valid JSON text.

## Installation

```bash
npm install coding-adventures-json-serializer
```

## Usage

### Serialize JsonValue to JSON text

```typescript
import { serialize, serializePretty } from "coding-adventures-json-serializer";
import { jsonObject, jsonString, jsonNumber } from "coding-adventures-json-value";

const value = jsonObject([
  ["name", jsonString("Alice")],
  ["age", jsonNumber(30)],
]);

// Compact
serialize(value);
// '{"name":"Alice","age":30}'

// Pretty
serializePretty(value);
// '{\n  "name": "Alice",\n  "age": 30\n}'
```

### Stringify native types

```typescript
import { stringify, stringifyPretty } from "coding-adventures-json-serializer";

// Compact
stringify({ name: "Alice", age: 30 });
// '{"name":"Alice","age":30}'

// Pretty with custom config
stringifyPretty({ name: "Alice", age: 30 }, {
  indentSize: 4,
  sortKeys: true,
  trailingNewline: true,
});
```

## Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `indentSize` | number | 2 | Spaces per indentation level |
| `indentChar` | string | `" "` | Indent character (space or tab) |
| `sortKeys` | boolean | false | Sort object keys alphabetically |
| `trailingNewline` | boolean | false | Add newline at end of output |

## API

| Function | Description |
|----------|-------------|
| `serialize(value)` | JsonValue to compact JSON text |
| `serializePretty(value, config?)` | JsonValue to pretty-printed JSON text |
| `stringify(native)` | Native JS types to compact JSON text |
| `stringifyPretty(native, config?)` | Native JS types to pretty-printed JSON text |

## Dependencies

- `coding-adventures-json-value` -- for JsonValue types and fromNative conversion

## License

MIT
