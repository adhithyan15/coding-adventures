# coding-adventures/lua/micro-qr

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

## What is Micro QR Code?

Micro QR Code is the compact sibling of regular QR Code, designed for applications where even the smallest standard QR (21×21) is too large — think surface-mount electronic component labels on circuit boards, miniature product markings, and tiny industrial tags scanned in controlled environments.

The defining structural difference: Micro QR uses a **single finder pattern** in the top-left corner, rather than regular QR's three corner finders. Because there is only one, orientation is always unambiguous — the data area is always to the bottom-right. This saves enormous space at the cost of requiring a controlled scanning environment.

## Symbol Sizes

| Symbol | Size    | Numeric cap | Alphanumeric cap | Byte cap |
|--------|---------|-------------|-----------------|----------|
| M1     | 11×11   | 5           | —               | —        |
| M2-L   | 13×13   | 10          | 6               | 4        |
| M2-M   | 13×13   | 8           | 5               | 3        |
| M3-L   | 15×15   | 23          | 14              | 9        |
| M3-M   | 15×15   | 18          | 11              | 7        |
| M4-L   | 17×17   | 35          | 21              | 15       |
| M4-M   | 17×17   | 30          | 18              | 13       |
| M4-Q   | 17×17   | 21          | 13              | 9        |

## Quick Start

```lua
local mqr = require("coding_adventures.micro_qr")

-- Auto-select smallest symbol and ECC level
local grid, err = mqr.encode("HELLO")
if err then error(err) end
print(grid.rows, grid.cols)   -- 13  13 (M2-L)
print(grid.version, grid.ecc) -- M2  L

-- Force a specific version and ECC level
local grid2, err2 = mqr.encode("A", {version = "M4", ecc = "Q"})

-- Access individual modules (1-indexed, true = dark)
for r = 1, grid.rows do
    for c = 1, grid.cols do
        io.write(grid.modules[r][c] and "■" or "□")
    end
    io.write("\n")
end
```

## API

| Symbol | Description |
|--------|-------------|
| `M.encode(input, options?)` | Encode string → grid or `nil, err` |
| `M.VERSION` | `"0.1.0"` |

### `M.encode(input, options?)`

Encodes a string to a Micro QR Code symbol.

**Parameters:**
- `input` (string) — the text to encode (UTF-8 / raw bytes)
- `options` (table, optional):
  - `version` — `"M1"`, `"M2"`, `"M3"`, or `"M4"` (nil = auto-select)
  - `ecc`     — `"DETECTION"`, `"L"`, `"M"`, or `"Q"` (nil = auto-select)

**Returns on success:**
```lua
{
  rows         = 11|13|15|17,
  cols         = 11|13|15|17,
  modules      = boolean[rows][cols],  -- true = dark, 1-indexed
  module_shape = "square",
  version      = "M1"|"M2"|"M3"|"M4",
  ecc          = "DETECTION"|"L"|"M"|"Q",
}
```

**Returns on failure:** `nil, error_message`

### Encoding mode selection (automatic)

The encoder picks the most compact mode that covers the full input:
1. **Numeric** — all digits `0–9` and symbol supports numeric mode
2. **Alphanumeric** — all chars in the 45-char set (`0-9 A-Z SP $%*+-./:`) and symbol supports it
3. **Byte** — any input encodable as raw bytes

### Error cases

- Input too long for any M1–M4 symbol at any ECC level
- Requested version+ECC combination that doesn't exist (e.g., `{version="M1", ecc="L"}`)
- Input that can't be encoded in any mode supported by the requested symbol

## In the Stack

```
(no Lua runtime dependencies — GF(256) arithmetic is self-contained)
         ↓
micro_qr  ← this package
```

## Key differences from regular QR Code

- Single finder pattern at top-left (not three corner patterns)
- Timing patterns at row 0 / col 0 (not row 6 / col 6)
- Only 4 mask patterns (not 8)
- Format XOR mask `0x4445` (not `0x5412`)
- Single copy of format information (not two)
- 2-module quiet zone (not 4)
- Narrower mode indicators: 0–3 bits (not 4)
- Single-block RS encoding (no interleaving)
