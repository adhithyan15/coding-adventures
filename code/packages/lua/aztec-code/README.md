# coding-adventures/lua/aztec-code

Aztec Code encoder — ISO/IEC 24778:2008 compliant.

## What is Aztec Code?

Aztec Code uses a single **bullseye finder pattern at the center** instead of QR's three corner patterns. No large quiet zone needed, and it's rotation-invariant.

Where it's used: IATA boarding passes, Eurostar/Amtrak rail tickets, European postal labels, US military ID cards.

## Quick Start

```lua
local aztec = require("coding_adventures.aztec_code")

local grid, err = aztec.encode("Hello, World!")
if err then error(err) end
print(grid.rows, grid.cols)  -- e.g. 19 19

-- With options
local grid2, err2 = aztec.encode("A", { min_ecc_percent = 33 })
```

## API

| Symbol | Description |
|--------|-------------|
| `M.encode(data, options?)` | Encode string → `{rows, cols, modules, compact, layers}` or `nil, err` |
| `M.VERSION` | `"0.1.0"` |
| `M.AztecError` | Error type string |
| `M.InputTooLongError` | Input-too-long error type string |

## In the Stack

```
(no Lua dependencies — all arithmetic is self-contained)
         ↓
aztec_code  ← this package
```
