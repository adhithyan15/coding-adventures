# coding-adventures/lua/pdf417

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

## What is PDF417?

A stacked linear barcode where every codeword has exactly 4 bars + 4 spaces occupying 17 modules. Each row is independently scannable by a laser. Used in driver's licences, boarding passes, shipping labels, immigration forms.

## Quick Start

```lua
local pdf417 = require("coding_adventures.pdf417")

local grid, err = pdf417.encode("Hello, PDF417!")
if err then error(err) end
print(grid.rows, grid.cols)

-- With options
local grid2, err2 = pdf417.encode("data", {
    ecc_level  = 3,
    columns    = 5,
    row_height = 4,
})
```

## API

| Symbol | Description |
|--------|-------------|
| `M.encode(data, opts?)` | Encode string → ModuleGrid or `nil, err` |
| `opts.ecc_level` | RS ECC level 0–8 (default: auto) |
| `opts.columns` | Data columns 1–30 (default: auto) |
| `opts.row_height` | Modules per row ≥1 (default: 3) |
| `M.VERSION` | `"0.1.0"` |
