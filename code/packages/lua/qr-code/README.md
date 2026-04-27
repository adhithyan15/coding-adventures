# coding-adventures-qr-code (Lua)

Lua implementation of a QR Code encoder compliant with ISO/IEC 18004:2015.
Supports versions 1–40, all four ECC levels (L/M/Q/H), and the three standard
encoding modes (numeric, alphanumeric, byte).

This is part of the `coding-adventures` monorepo math/encoding stack:

```
MA01 gf256          — GF(2^8) field arithmetic
MA02 reed-solomon   — RS error-correcting codes
                               ↓
          coding-adventures-qr-code (THIS PACKAGE)
                               ↓
          coding-adventures-barcode-2d — layout() → PaintScene
                               ↓
          paint backend (ASCII, SVG, Metal, …)
```

## Installation

```bash
luarocks install coding-adventures-qr-code
```

## Usage

```lua
local qr = require("coding_adventures.qr_code")

-- Encode a URL at ECC level M (default)
local grid, err = qr.encode("https://example.com", "M")
if err then error(err.message) end

-- grid.rows == grid.cols == 25 (version 2 at M for this URL)
-- grid.modules[r][c] == true means dark module, false means light

-- Render via barcode-2d + a paint backend
local b2d = require("coding_adventures.barcode_2d")
local scene = b2d.layout(grid)
-- pass scene to your chosen paint backend
```

## API

### `qr.encode(data, level) -> grid, err`

Encode a UTF-8 string to a QR Code module grid.

| Parameter | Type   | Description                                        |
|-----------|--------|----------------------------------------------------|
| `data`    | string | The data to encode (UTF-8; treated as raw bytes)   |
| `level`   | string | ECC level: `"L"`, `"M"` (default), `"Q"`, `"H"`   |

**Returns** `grid, nil` on success or `nil, err` on failure.

The `grid` table:

| Field          | Type          | Description                                          |
|----------------|---------------|------------------------------------------------------|
| `rows`         | number        | Grid height (= `4*version + 17`)                     |
| `cols`         | number        | Grid width (same as `rows`)                          |
| `modules`      | `bool[][]`    | `modules[r][c]` — `true` = dark, 1-indexed           |
| `module_shape` | `"square"`    | Always `"square"` for QR Code                        |
| `version`      | number        | QR version selected (1–40)                           |
| `ecc_level`    | string        | ECC level used (`"L"/"M"/"Q"/"H"`)                   |

The `err` table on failure:

| Field     | Type   | Description                              |
|-----------|--------|------------------------------------------|
| `kind`    | string | `"InputTooLongError"` or `"QRCodeError"` |
| `message` | string | Human-readable error description         |

## ECC Levels

| Level | Recovery | Typical use                              |
|-------|----------|------------------------------------------|
| L     | ~7%      | Maximum data density, clean environments |
| M     | ~15%     | General-purpose (common default)         |
| Q     | ~25%     | Moderate noise or damage expected        |
| H     | ~30%     | High damage risk, overlaid logos         |

## Encoding Modes

The encoder automatically selects the most compact mode:

| Mode         | Characters                              | Bits/char |
|--------------|-----------------------------------------|-----------|
| Numeric      | `0–9`                                   | ~3.3      |
| Alphanumeric | `0–9`, `A–Z`, `space`, `$%*+-./:` (45) | ~5.5      |
| Byte         | Any UTF-8 string                        | 8.0       |

## Running Tests

```bash
cd code/packages/lua/qr-code
mise exec -- busted spec/ --verbose
```

## Version Capacity Reference (ECC M)

| Version | Size   | Numeric | Alphanumeric | Byte |
|---------|--------|---------|--------------|------|
| 1       | 21×21  | 34      | 20           | 14   |
| 2       | 25×25  | 63      | 38           | 26   |
| 5       | 37×37  | 154     | 93           | 64   |
| 10      | 57×57  | 395     | 239          | 163  |
| 20      | 97×97  | 1000    | 606          | 412  |
| 40      | 177×177| 4296    | 2604         | 1777 |

## Dependencies

- `lua >= 5.4`
- `coding-adventures-gf256` — GF(256) arithmetic for RS ECC computation
- `coding-adventures-barcode-2d` — for `layout()` rendering (optional for raw grid use)
