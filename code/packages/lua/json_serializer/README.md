# coding-adventures-json-serializer (Lua)

Schema-aware JSON serializer/deserializer built on top of `json_value`.

## What it does

`json_serializer` extends `json_value` with four higher-level operations:

| Function | Purpose |
|---|---|
| `encode(value, opts)` | Serialize with indent, sort_keys, allow_nan, max_depth |
| `decode(json_str, opts)` | Parse with comment stripping and trailing-comma tolerance |
| `validate(value, schema)` | Validate a native Lua value against a JSON Schema subset |
| `schema_encode(value, schema)` | Encode with type coercion and property filtering |

## Where it fits in the stack

```
json_serializer   ← this package
      ↓
 json_value        (native Lua value round-trip)
      ↓
 json_parser, json_lexer, parser, lexer, grammar_tools, state_machine, directed_graph
```

## Usage

```lua
local js = require("coding_adventures.json_serializer")

-- Pretty-print with sorted keys
print(js.encode({b=2, a=1}, {indent=2, sort_keys=true}))
-- {
--   "a": 1,
--   "b": 2
-- }

-- Decode JSONC (with comments)
local v = js.decode([[
{
  "name": "Alice", // user's name
  "age": 30
}]], {allow_comments = true})
print(v.name)  -- Alice

-- Decode with trailing commas
local arr = js.decode("[1, 2, 3,]")
print(arr[3])  -- 3

-- Validate against a schema
local schema = {
  type = "object",
  required = {"name", "age"},
  properties = {
    name = {type = "string", minLength = 1},
    age  = {type = "integer", minimum = 0, maximum = 150},
  },
}
local ok, errs = js.validate({name = "Alice", age = 30}, schema)
print(ok)   -- true

-- Schema-guided encoding (coerce numbers to strings, drop extra fields)
local api_schema = {
  type = "object",
  additional_properties = false,
  properties = {
    price = {type = "string"},
    qty   = {type = "number"},
  },
}
local s = js.schema_encode({price = 9.99, qty = 3, internal = "secret"}, api_schema)
-- {"price":"9.99","qty":3}   (internal field dropped, price coerced to string)
```

## Options

### `encode(value, opts)`

| Option | Type | Default | Description |
|---|---|---|---|
| `indent` | number | 0 | Spaces per indent level (0 = compact) |
| `sort_keys` | boolean | true | Sort object keys alphabetically |
| `allow_nan` | boolean | false | Emit NaN/Infinity as quoted strings instead of null |
| `max_depth` | number | 100 | Raise error if nesting exceeds this depth |

### `decode(json_str, opts)`

| Option | Type | Default | Description |
|---|---|---|---|
| `allow_comments` | boolean | false | Strip `//` and `/* */` comments before parsing |
| `strict` | boolean | false | When false, strip trailing commas; when true, reject them |

### `validate(value, schema)`

Supported schema keywords: `type`, `properties`, `required`, `additional_properties`, `items`, `minItems`, `maxItems`, `minimum`, `maximum`, `minLength`, `maxLength`, `pattern`, `enum`.

Returns `true, nil` on success; `false, errors_table` on failure (all errors collected, not just the first).

### `schema_encode(value, schema, opts)`

Applies coercions (number → string when schema says `type="string"`) and filters unknown properties when `additional_properties=false`, then calls `encode`.

## Running the tests

```sh
# From the package directory
luarocks make --local coding-adventures-json-serializer-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
```

Or use the BUILD file with the monorepo build tool.
