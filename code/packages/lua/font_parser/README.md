# coding-adventures-font-parser (Lua)

A metrics-only OpenType/TrueType font parser in pure Lua 5.4 with **zero
runtime dependencies**. Part of the FNT series (see
[`code/specs/FNT00-font-parser.md`](../../../../specs/FNT00-font-parser.md)).

## Installation

```
luarocks install coding-adventures-font-parser
```

Or locally:

```
luarocks make coding-adventures-font-parser-0.1.0-1.rockspec --local
```

## Quick start

```lua
local fp = require("coding_adventures.font_parser")

-- 1. Load font bytes ("rb" = binary mode)
local fh   = assert(io.open("Inter-Regular.ttf", "rb"))
local data = fh:read("*a")
fh:close()

local font = fp.load(data)

-- 2. Global metrics
local m = fp.font_metrics(font)
print(m.units_per_em)     -- 2048
print(m.family_name)      -- "Inter"
print(m.ascender)         -- 1984
print(m.descender)        -- -494
print(m.x_height)         -- 1082  (nil if OS/2 version < 2)
print(m.cap_height)       -- 1456  (nil if OS/2 version < 2)

-- 3. Glyph lookup
local gid_a = fp.glyph_id(font, 0x0041)   -- 'A'
local gid_v = fp.glyph_id(font, 0x0056)   -- 'V'

-- 4. Per-glyph metrics
local gm = fp.glyph_metrics(font, gid_a)
print(gm.advance_width)      -- e.g. 1401
print(gm.left_side_bearing)  -- e.g. 7

-- 5. Kerning (0 when no kern table or pair absent)
print(fp.kerning(font, gid_a, gid_v))
```

## Error handling

`load` raises a table on failure; catch it with `pcall`:

```lua
local ok, err = pcall(fp.load, data)
if not ok then
  print(err.kind)     -- "BufferTooShort" | "InvalidMagic" | "TableNotFound" | "ParseError"
  print(err.message)  -- human-readable description
end
```

## API reference

### `fp.load(data) → FontFile`

Parse a binary font string. Raises `{kind, message}` on failure.

### `fp.font_metrics(font) → table`

Returns a table with `units_per_em`, `ascender`, `descender`, `line_gap`,
`x_height` (nil if absent), `cap_height` (nil if absent), `num_glyphs`,
`family_name`, `subfamily_name`.

### `fp.glyph_id(font, codepoint) → integer | nil`

Returns `nil` for unmapped or out-of-BMP codepoints.

### `fp.glyph_metrics(font, glyph_id) → {advance_width, left_side_bearing} | nil`

Returns `nil` for out-of-range glyph IDs.

### `fp.kerning(font, left, right) → integer`

Returns the kern value or `0`.

## Development

```
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://lunarmodules.github.io/busted/) (`luarocks install busted`).

## License

MIT
