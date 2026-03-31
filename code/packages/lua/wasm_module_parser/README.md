# coding-adventures-wasm-module-parser

WebAssembly binary module parser for Lua.

Parses `.wasm` binary files into structured Lua tables following the
[WebAssembly binary format specification](https://webassembly.github.io/spec/core/binary/modules.html).

## Part of the coding-adventures stack

This library sits at the WebAssembly tooling layer. It depends on:
- `coding-adventures-wasm-leb128` — LEB128 variable-length integer decoding
- `coding-adventures-wasm-types` — WebAssembly value type definitions

It is used by higher-level tools that need to inspect or transform Wasm
binaries (linkers, analyzers, interpreters, etc.).

## Installation

```bash
luarocks make --local coding-adventures-wasm-module-parser-0.1.0-1.rockspec
```

## Usage

```lua
local parser = require("coding_adventures.wasm_module_parser")

-- Parse a .wasm file
local f = io.open("my_module.wasm", "rb")
local bytes = f:read("*all")
f:close()

local module = parser.parse(bytes)

-- Inspect the module
print("Version:", module.version)           -- 1
print("Types:", #module.types)              -- number of function signatures
print("Imports:", #module.imports)          -- number of imports
print("Exports:", #module.exports)          -- number of exports
print("Functions:", #module.functions)      -- number of local functions
print("Code entries:", #module.codes)       -- number of function bodies

-- Examine type signatures
for i, ft in ipairs(module.types) do
    local params = table.concat(ft.params, ", ")
    local results = table.concat(ft.results, ", ")
    print(string.format("Type %d: (%s) -> (%s)", i-1, params, results))
end

-- Examine exports
for _, exp in ipairs(module.exports) do
    print(string.format("Export: %s (%s %d)",
        exp.name, exp.desc.kind, exp.desc.idx))
end

-- Find a section
local types = parser.get_section(module, parser.SECTION_TYPE)
```

## Module Structure

The `parse()` function returns a table with these fields:

| Field       | Type    | Description |
|-------------|---------|-------------|
| `magic`     | string  | Always `"\0asm"` |
| `version`   | integer | Always `1` |
| `types`     | array   | Function type signatures `{params, results}` |
| `imports`   | array   | Imported symbols `{mod, name, desc}` |
| `functions` | array   | Type indices for local functions |
| `tables`    | array   | Table definitions `{ref_type, limits}` |
| `memories`  | array   | Memory definitions `{limits}` |
| `globals`   | array   | Global variable definitions |
| `exports`   | array   | Exported symbols `{name, desc}` |
| `start`     | integer | Start function index (or nil) |
| `elements`  | array   | Element segment raw bytes |
| `codes`     | array   | Function bodies `{locals, body}` |
| `data`      | array   | Data segment raw bytes |
| `custom`    | array   | Custom sections `{name, data}` |

## Section ID Constants

```lua
parser.SECTION_CUSTOM   -- 0
parser.SECTION_TYPE     -- 1
parser.SECTION_IMPORT   -- 2
parser.SECTION_FUNCTION -- 3
parser.SECTION_TABLE    -- 4
parser.SECTION_MEMORY   -- 5
parser.SECTION_GLOBAL   -- 6
parser.SECTION_EXPORT   -- 7
parser.SECTION_START    -- 8
parser.SECTION_ELEMENT  -- 9
parser.SECTION_CODE     -- 10
parser.SECTION_DATA     -- 11
```

## API Reference

### `parser.parse(bytes_string) → module`

Parse a Wasm binary string. Errors if the magic number or version are invalid.

### `parser.parse_header(bytes_array, pos) → new_pos`

Validate the 8-byte Wasm header. `bytes_array` is a 1-indexed integer array.

### `parser.parse_section(bytes_array, pos) → section_info, content_start`

Parse one section header (ID + length). Does not parse the content.

### `parser.get_section(module, section_id) → section_data`

Retrieve a parsed section from a module by its ID constant.

## License

MIT
