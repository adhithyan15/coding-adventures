# coding_adventures_font_parser (Elixir)

A metrics-only OpenType/TrueType font parser in pure Elixir with **zero
runtime dependencies**. Part of the FNT series (see
[`code/specs/FNT00-font-parser.md`](../../../../specs/FNT00-font-parser.md)).

## Installation

```elixir
# mix.exs
defp deps do
  [{:coding_adventures_font_parser, "~> 0.1"}]
end
```

## Quick start

```elixir
alias CodingAdventures.FontParser, as: FP

# 1. Load font bytes (File.read! returns a binary)
bytes = File.read!("/path/to/Inter-Regular.ttf")
font  = FP.load(bytes)

# 2. Global metrics
m = FP.font_metrics(font)
m.units_per_em    # => 2048
m.family_name     # => "Inter"
m.ascender        # => 1984
m.descender       # => -494
m.x_height        # => 1082  (nil if OS/2 version < 2)
m.cap_height      # => 1456  (nil if OS/2 version < 2)

# 3. Glyph lookup
gid_a = FP.glyph_id(font, 0x0041)  # 'A'
gid_v = FP.glyph_id(font, 0x0056)  # 'V'

# 4. Per-glyph metrics
gm = FP.glyph_metrics(font, gid_a)
gm.advance_width      # e.g. 1401
gm.left_side_bearing  # e.g. 7

# 5. Kerning (0 if font uses GPOS or pair is absent)
FP.kerning(font, gid_a, gid_v)
```

## API

### `load(data :: binary()) :: FontFile.t()`

Raises `FontError` on failure. Check `error.kind` for the category:

| kind | Meaning |
|------|---------|
| `"BufferTooShort"` | Binary too short |
| `"InvalidMagic"` | Unknown sfntVersion |
| `"TableNotFound"` | Required table missing |
| `"ParseError"` | Table structurally invalid |

### `font_metrics(font) :: FontMetrics.t()`

Returns a `FontMetrics` struct (see module for field types).

### `glyph_id(font, codepoint :: integer()) :: non_neg_integer() | nil`

Returns `nil` for unmapped codepoints or out-of-BMP values.

### `glyph_metrics(font, glyph_id :: integer()) :: GlyphMetrics.t() | nil`

Returns `nil` for out-of-range glyph IDs.

### `kerning(font, left :: integer(), right :: integer()) :: integer()`

Returns the kern value or `0`.

## Implementation notes

- All font integers are big-endian; Elixir binary patterns
  (`<<v::unsigned-big-16>>`) handle this natively.
- All ranges use explicit step `//1` to prevent silent direction flips
  when the count is zero.
- `cmap` Format 4 `idRangeOffset` is a self-relative C pointer, implemented
  as: `abs_off = iro_abs + iro + (cp - start_code) * 2`.
- `kern` Format 0 coverage: format number is in bits 8-15 (`coverage >>> 8`).
- UTF-16 BE decoding uses `:unicode.characters_to_binary/3`.

## Development

```
mix deps.get
mix test --cover
```

## License

MIT
