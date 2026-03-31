# coding-adventures-json-value

A JSON value evaluator for the coding-adventures monorepo. It walks the Abstract Syntax Tree produced by `json_parser` and converts JSON values into native Lua data structures (tables, strings, numbers, booleans). It also serializes native Lua values back to JSON strings.

## What it does

Given the AST for `{"name": "Alice", "age": 30}`, the evaluator produces:

```lua
{
  name = "Alice",
  age  = 30,
}
```

The reverse direction:

```lua
jv.to_json({ name = "Alice", age = 30 })
-- → '{"age":30,"name":"Alice"}'
```

## How it fits in the stack

```
json_value  ← this package
     ↓
json_parser  (provides AST)
     ↓
parser, grammar_tools, json_lexer, lexer, state_machine, directed_graph
```

## Usage

```lua
local jv = require("coding_adventures.json_value")

-- One-step parse + evaluate
local t = jv.from_string('{"name": "Alice", "age": 30}')
print(t.name)   -- Alice
print(t.age)    -- 30

-- JSON null
local v = jv.from_string("null")
print(jv.is_null(v))    -- true
print(v == jv.null)     -- true

-- Serialize to JSON (compact)
print(jv.to_json({x = 1, y = 2}))
-- → {"x":1,"y":2}

-- Serialize to JSON (pretty, 2-space indent)
print(jv.to_json({x = 1, y = 2}, 2))
-- → {
--     "x": 1,
--     "y": 2
--   }

-- Round-trip
local original = '{"tags":["lua","json"],"ok":true}'
local v = jv.from_string(original)
local back = jv.to_json(v)
-- back → '{"ok":true,"tags":["lua","json"]}'  (keys sorted alphabetically)
```

## API

### `jv.null`

A unique sentinel table representing JSON `null`. Use `jv.is_null(v)` to test for it.

### `jv.is_null(v) → boolean`

Returns `true` when `v` is the `jv.null` sentinel.

### `jv.evaluate(ast) → any`

Walk a JSON AST (from `json_parser.parse`) and return the native Lua value.

### `jv.from_string(json_str) → any`

Parse a JSON string and evaluate it in one step. Raises on invalid input.

### `jv.to_json(value, indent) → string`

Serialize a native Lua value to a JSON string. `indent` (optional, default 0) enables pretty-printing with that many spaces per level. Keys are sorted alphabetically for deterministic output.

## Type mapping

| JSON type | Lua type              |
|-----------|-----------------------|
| object    | table (string keys)   |
| array     | table (integer keys)  |
| string    | string                |
| number    | number                |
| boolean   | boolean               |
| null      | `jv.null` (sentinel)  |

## Null sentinel note

Lua's `nil` cannot be stored in a table — `t[k] = nil` removes the key. JSON `null` must survive inside a Lua table (e.g. `{"x": null}`). The `jv.null` sentinel is a unique empty table that can be stored as a table value and tested with `jv.is_null(v)`.

## Version

0.1.0
