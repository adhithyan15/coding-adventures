# qr-code (Go)

A complete QR Code encoder for Go, ISO/IEC 18004:2015 compliant.

QR Code (Quick Response code) was invented by Masahiro Hara at Denso Wave in
1994 to track automotive parts. It is now the most widely deployed 2D barcode
format on earth — on every product label, restaurant menu, bus stop, and
business card.

This package encodes any UTF-8 string into a scannable QR Code. It does NOT
implement decoding (that requires image preprocessing and perspective correction).

## How it fits in the stack

```
barcode-2d (ModuleGrid + Layout) ← qr-code depends on this
gf256      (GF(2^8) arithmetic)  ← qr-code depends on this

input string → qr-code → ModuleGrid → barcode-2d.Layout → PaintScene → SVG / Metal / terminal
```

`qr-code` sits at layer MA03 in the coding-adventures math stack. It depends on
MA01 (`gf256`) for Galois Field arithmetic and on the `barcode-2d` infrastructure
package for the `ModuleGrid` type and `Layout` function.

## Encoding pipeline

```
input string
  → mode selection     (numeric / alphanumeric / byte — most compact mode)
  → version selection  (smallest symbol that holds the data at chosen ECC level)
  → bit stream         (mode indicator + char count + data + padding)
  → blocks + RS ECC   (GF(256) Reed-Solomon, b=0 root convention)
  → interleave         (weave codewords across all blocks)
  → grid init          (finder patterns, timing strips, alignment patterns, dark module)
  → zigzag placement   (fill data modules bottom-right to top-left, snake pattern)
  → mask evaluation    (try all 8 masks, pick the one with the lowest penalty score)
  → finalize           (write format info + version info for v7+)
  → ModuleGrid         (abstract boolean grid: true = dark module)
```

## Usage

```go
import qrcode "github.com/adhithyan15/coding-adventures/code/packages/go/qr-code"

// Encode a URL at medium error correction.
grid, err := qrcode.Encode("https://example.com", qrcode.EncodeOptions{
    Level: qrcode.EccM,
})
if err != nil {
    log.Fatal(err)
}
// grid.Rows == grid.Cols == 25  (version 2, 25×25 modules)
// grid.Modules[r][c] == true means dark module at row r, col c

// Encode directly to a PaintScene (for rendering).
import barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"

scene, err := qrcode.EncodeToScene(
    "HELLO WORLD",
    qrcode.EncodeOptions{Level: qrcode.EccM},
    barcode2d.DefaultBarcode2DLayoutConfig,
)
```

## Error correction levels

| Level | Recovery | Use case                        |
|-------|----------|---------------------------------|
| L     | ~7%      | Maximum data density             |
| M     | ~15%     | General-purpose (common default) |
| Q     | ~25%     | Moderate noise/damage expected   |
| H     | ~30%     | High damage risk, logo overlaid  |

## Encoding modes

| Mode          | Characters           | Bits/char |
|---------------|----------------------|-----------|
| Numeric       | 0–9                  | ~3.3      |
| Alphanumeric  | 0–9, A–Z, `$%*+-./: ` | ~5.5    |
| Byte          | Any UTF-8 bytes      | 8         |

The encoder automatically selects the most compact mode that covers the entire
input. Mixed-mode encoding (different segments in different modes) is planned for v0.2.0.

## Version and capacity

QR symbols come in 40 versions. Version 1 is 21×21 modules; version 40 is
177×177. The encoder automatically picks the minimum version that fits.

Selected byte-mode capacities at ECC level M:

| Version | Size   | Bytes |
|---------|--------|-------|
| 1       | 21×21  | 16    |
| 2       | 25×25  | 28    |
| 5       | 37×37  | 86    |
| 10      | 57×57  | 213   |
| 20      | 97×97  | 666   |
| 40      | 177×177 | 2331 |

## Reed-Solomon error correction

QR uses Reed-Solomon over GF(256) with the **b=0 convention**:
```
g(x) = (x + α^0)(x + α^1)···(x + α^{n-1})
```
where α = 2 and n is the number of ECC codewords per block. This package
implements its own RS encoder (not MA02) because QR uses b=0 while MA02 uses b=1.

## API

```go
// Encode encodes a string into a QR Code ModuleGrid.
func Encode(data string, opts EncodeOptions) (barcode2d.ModuleGrid, error)

// EncodeToScene encodes directly to a pixel-resolved PaintScene.
func EncodeToScene(data string, opts EncodeOptions, cfg barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error)

// EncodeOptions controls encoding behaviour.
type EncodeOptions struct {
    Level   EccLevel    // EccL, EccM, EccQ, EccH
    Version int         // 0 = auto-select, 1-40 = forced
    Mode    EncodingMode // ModeAuto, ModeNumeric, ModeAlphanumeric, ModeByte
}

// Error types
type InputTooLongError  // input does not fit in any version at the chosen ECC
type InvalidInputError  // input contains characters invalid for the chosen mode
```

## Testing

```
go test ./... -v -cover
```

Coverage: 94.4% (47 tests covering all pipeline stages, error paths, and invariants).
