# micro-qr (Go)

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

## What is Micro QR Code?

Micro QR Code is the compact sibling of regular QR Code, designed for applications
where even the smallest standard QR (21×21 at version 1) is too large. Common uses
include surface-mount component labels on circuit boards, miniature product markings,
and tiny industrial tags scanned in controlled environments.

The defining structural difference: Micro QR uses a **single finder pattern** in the
top-left corner, rather than regular QR's three corner finders. This saves dramatic
space at the cost of some scanning robustness — Micro QR targets factory-floor
environments, not consumer smartphones.

## Symbol sizes

```
M1: 11×11 modules   (numeric only, error detection)
M2: 13×13 modules   (numeric, alphanumeric, byte — L and M ECC)
M3: 15×15 modules   (numeric, alphanumeric, byte — L and M ECC)
M4: 17×17 modules   (numeric, alphanumeric, byte — L, M, and Q ECC)

formula: size = 2 × version_number + 9
```

## Key differences from regular QR Code

| Property               | Regular QR Code    | Micro QR Code         |
|------------------------|--------------------|-----------------------|
| Finder patterns        | 3 (three corners)  | 1 (top-left only)     |
| Timing pattern row/col | Row 6, col 6       | Row 0, col 0          |
| Mask patterns          | 8                  | 4                     |
| Format info XOR mask   | 0x5412             | 0x4445                |
| Format info copies     | 2                  | 1                     |
| Quiet zone modules     | 4                  | 2                     |
| Mode indicator width   | 4 bits             | 0–3 bits (by version) |
| RS interleaving        | Multi-block        | Single block only     |

## Where this fits in the stack

```
Input string
  → micro-qr.Encode()     ← this package
  → barcode2d.ModuleGrid
  → barcode-2d.Layout()
  → paint-instructions.PaintScene
  → paint-vm (SVG, terminal, Metal, …)
```

## Usage

```go
import (
    microqr "github.com/adhithyan15/coding-adventures/code/packages/go/micro-qr"
)

// Auto-select: "HELLO" → M2 alphanumeric (13×13)
grid, err := microqr.Encode("HELLO", nil, nil)
if err != nil {
    log.Fatal(err)
}
fmt.Printf("Symbol: %d×%d modules\n", grid.Rows, grid.Cols)

// Force a specific version and ECC level
grid, err = microqr.EncodeAt("12345", microqr.VersionM1, microqr.EccDetection)

// Convert to a PaintScene for rendering
scene, err := microqr.Layout(grid, nil) // uses 2-module quiet zone by default
```

## Supported ECC levels

| Level       | Symbol(s)   | Approximate recovery |
|-------------|-------------|----------------------|
| Detection   | M1 only     | Error detection only |
| L (Low)     | M2, M3, M4  | ~7% of codewords     |
| M (Medium)  | M2, M3, M4  | ~15% of codewords    |
| Q (Quartile)| M4 only     | ~25% of codewords    |

Level H (High) is not available in any Micro QR symbol.

## Encoding pipeline

```
input string
  → selectConfig()    find smallest (version, ECC) that fits
  → selectMode()      numeric > alphanumeric > byte
  → buildDataCodewords()  mode indicator + char count + data + terminator + padding
  → rsEncode()        Reed-Solomon ECC over GF(256)/0x11D, b=0 convention
  → buildGrid()       finder + L-shaped separator + timing + format reservation
  → placeBits()       two-column zigzag from bottom-right
  → evaluate 4 masks, pick lowest penalty score
  → writeFormatInfo() 15 bits, single copy, XOR 0x4445
  → barcode2d.ModuleGrid
```

## Dependencies

- `barcode-2d` — `ModuleGrid` type and `Layout()` function
- `gf256` — Galois Field GF(2^8) `Multiply()` used by the RS encoder
- `paint-instructions` — `PaintScene` type returned by `Layout()`

## Testing

```bash
go test ./... -v -cover
go vet ./...
```

Current coverage: **95.9% of statements**.
