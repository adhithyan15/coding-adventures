# coding-adventures-data-matrix (Lua)

Lua implementation of a Data Matrix ECC200 encoder, conforming to ISO/IEC
16022:2006. Supports all 30 square symbol sizes (10×10 through 144×144)
and all 6 rectangular sizes (8×18 through 16×48).

This is part of the `coding-adventures` monorepo 2D-barcode stack:

```
MA01 gf256          — GF(2^8) field arithmetic (general)
MA02 reed-solomon   — Reed-Solomon error correction (general)
                               ↓
          coding-adventures-data-matrix (THIS PACKAGE)
                               ↓
          coding-adventures-barcode-2d — layout() → PaintScene
                               ↓
          paint backend (ASCII, SVG, Metal, …)
```

Data Matrix uses its own GF(256) field (polynomial `0x12D`) and the
Reed-Solomon `b=1` convention (roots α¹…αⁿ). This is **not the same**
field that QR Code uses (`0x11D`, `b=0`), so we build local log/antilog
tables in this module rather than reusing the shared `gf256` package.

## Installation

```bash
luarocks install coding-adventures-data-matrix
```

## Usage

```lua
local dm = require("coding_adventures.data_matrix")

local grid, err = dm.encode("HELLO")
if err then error(err.message) end

-- grid.rows == grid.cols == 14   (5 ASCII codewords → 14×14 symbol)
-- grid.modules[r][c] == true   means dark, false means light  (1-indexed)

-- Render via barcode-2d + a paint backend
local b2d = require("coding_adventures.barcode_2d")
local scene = b2d.layout(grid, { quiet_zone_modules = 1 })
-- pass scene to your chosen paint backend
```

## API

### `dm.encode(data, opts) -> grid, err`

Encode a string into a Data Matrix ECC200 module grid.

| Parameter        | Type            | Description                                                   |
|------------------|-----------------|---------------------------------------------------------------|
| `data`           | string          | The data to encode (treated as raw bytes)                     |
| `opts`           | table or nil    | Optional encoder options                                      |
| `opts.shape`     | string          | `"square"` (default), `"rectangular"`, or `"any"`             |

**Returns** `grid, nil` on success or `nil, err` on failure.

The `grid` table:

| Field          | Type        | Description                                       |
|----------------|-------------|---------------------------------------------------|
| `rows`         | number      | Symbol height in modules                          |
| `cols`         | number      | Symbol width in modules                           |
| `modules`      | `bool[][]`  | `modules[r][c]` — `true` = dark, 1-indexed        |
| `module_shape` | `"square"`  | Always `"square"` for Data Matrix                 |
| `symbol_rows`  | number      | Echo of `rows`                                    |
| `symbol_cols`  | number      | Echo of `cols`                                    |
| `data_cw`      | number      | Data codeword capacity used                       |
| `ecc_cw`       | number      | ECC codeword capacity used                        |

The `err` table on failure:

| Field      | Type    | Description                                       |
|------------|---------|---------------------------------------------------|
| `kind`     | string  | `"InputTooLongError"` or `"DataMatrixError"`      |
| `message`  | string  | Human-readable error description                  |
| `encoded`  | number  | (InputTooLong only) codewords produced            |
| `max`      | number  | (InputTooLong only) maximum (1558)                |

## How Data Matrix Differs From QR Code

| Aspect             | QR Code                    | Data Matrix ECC200             |
|--------------------|----------------------------|--------------------------------|
| Field polynomial   | 0x11D                      | 0x12D                          |
| RS convention      | b=0 (roots α⁰…α^{n-1})     | b=1 (roots α¹…αⁿ)              |
| Finder             | Three corner finders       | L-shape (left + bottom)        |
| Timing             | Strips between finders     | Top + right edges              |
| Placement          | Snake from bottom-right    | Diagonal Utah algorithm        |
| Masking            | 8 patterns, 4-rule penalty | None — diagonal walk suffices  |
| Symbol sizes       | 40 versions, all square    | 30 squares + 6 rectangles      |

## Symbol Capacity Reference (square)

| Symbol Size | Data CW | ECC CW | Blocks | Typical Use         |
|-------------|---------|--------|--------|---------------------|
| 10×10       | 3       | 5      | 1      | "A", short tags     |
| 14×14       | 8       | 10     | 1      | UPC numbers         |
| 20×20       | 22      | 18     | 1      | Lot codes           |
| 26×26       | 44      | 28     | 1      | Serial numbers      |
| 32×32       | 62      | 36     | 2      | URLs                |
| 52×52       | 204     | 84     | 4      | GS1 healthcare      |
| 104×104     | 816     | 336    | 6      | Aerospace marking   |
| 144×144     | 1558    | 620    | 10     | Maximum capacity    |

## Running Tests

```bash
cd code/packages/lua/data-matrix
mise exec -- busted spec/ --verbose
```

## Dependencies

- `lua >= 5.4`

(All GF(256)/0x12D arithmetic is built locally; no runtime dependency on
`coding-adventures-gf256`. The `barcode-2d` module is only needed if you
want to render the output to pixels.)

## References

- ISO/IEC 16022:2006 — Information technology — Automatic identification
  and data capture techniques — Data Matrix bar code symbology specification.
- Annex F: worked examples (used to validate "A" → 10×10 in our test suite).
