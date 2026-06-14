# coding-adventures/go/pdf417

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

## What is PDF417?

PDF417 (Portable Data File 417) is a stacked linear barcode invented by Ynjiun P. Wang at Symbol Technologies in 1991. The "417" refers to its structure: every codeword has exactly 4 bars and 4 spaces occupying 17 horizontal modules.

Unlike true 2D matrix barcodes (QR, Data Matrix), PDF417 is a stack of short 1D rows — each independently scannable by a moving laser.

Where it's deployed:
- **AAMVA** — North American driver's licences and government IDs
- **IATA BCBP** — airline boarding passes
- **USPS** — domestic shipping labels
- **US immigration** — Form I-94, customs declarations
- **Healthcare** — patient wristbands, medication labels

## Quick Start

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/pdf417"

// Simple encode (auto ECC, auto dimensions)
grid := pdf417.Encode("Hello, PDF417!")

// Full control
grid, err := pdf417.EncodeBytes(data, pdf417.Options{
    ECCLevel:  3,
    Columns:   5,
    RowHeight: 4,
})

// Produce a PaintScene for rendering
scene, err := pdf417.EncodeToScene(data, pdf417.Options{ECCLevel: pdf417.ECCLevelAuto}, nil)
```

## API

| Symbol | Description |
|--------|-------------|
| `Encode(data string) *ModuleGrid` | Encode string with defaults |
| `EncodeBytes(data []byte, opts Options) (*ModuleGrid, error)` | Full control |
| `EncodeToScene(data []byte, opts Options, cfg *Barcode2DLayoutConfig) (PaintScene, error)` | Encode + layout |
| `Options.ECCLevel` | RS ECC level 0–8, or `ECCLevelAuto` |
| `Options.Columns` | Data columns 1–30 (0 = auto) |
| `Options.RowHeight` | Modules per row 1–10 (0 = default 3) |

## In the Stack

```
paint-instructions   ← PaintScene rendering target
barcode-2d           ← ModuleGrid, layout engine
         ↓
pdf417               ← this package
```
