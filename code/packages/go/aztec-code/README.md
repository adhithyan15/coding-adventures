# aztec-code — Go

ISO/IEC 24778:2008 Aztec Code encoder for the coding-adventures monorepo.

## What is Aztec Code?

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
published as a patent-free format. Unlike QR Code (which uses three finder
squares at three corners), Aztec places a single **bullseye finder pattern at
the center**. A scanner finds the center first, then reads outward in a
clockwise spiral — no large quiet zone is needed.

### Where Aztec Code is used

- **IATA boarding passes** — every airline boarding pass
- **Eurostar, Amtrak, TGV** — rail tickets (printed and on-screen)
- **PostNL, Deutsche Post, La Poste** — postal routing labels
- **US military ID cards**

## Symbol variants

```
Compact: 1–4 layers,  size = 11 + 4 × layers  (15×15 to 27×27)
Full:    1–32 layers, size = 15 + 4 × layers  (19×19 to 143×143)
```

The encoder automatically selects the smallest symbol that fits the data.

## Quick start

```go
import azteccode "github.com/adhithyan15/coding-adventures/code/packages/go/aztec-code"

// Encode a string — auto-selects smallest symbol at 23% ECC.
grid, err := azteccode.Encode("Hello, Aztec!", nil)
if err != nil {
    log.Fatal(err)
}
fmt.Printf("Symbol is %d×%d modules\n", grid.Rows, grid.Cols)

// Access the module grid (true = dark, false = light).
for row := range grid.Modules {
    for col, dark := range grid.Modules[row] {
        _ = dark
    }
}

// Encode raw bytes.
data := []byte{0x48, 0x65, 0x6C, 0x6C, 0x6F}
grid, err = azteccode.EncodeBytes(data, nil)

// Encode with custom ECC.
opts := &azteccode.Options{MinEccPercent: 33}
grid, err = azteccode.Encode("Hello!", opts)

// Encode and convert to a PaintScene for rendering.
scene, err := azteccode.EncodeToScene("Hello!", nil, barcode2d.Barcode2DLayoutConfig{})
```

## Encoding pipeline (v0.1.0)

1. Input bytes encoded via Binary-Shift escape from Upper mode (byte-mode only).
2. Smallest symbol selected: compact 1–4 layers, then full 1–32 layers.
3. Data codewords padded to exact slot count; last zero codeword → 0xFF.
4. Reed-Solomon ECC over GF(256)/0x12D (b=1 convention, same as Data Matrix).
5. Bit stuffing: complement bit inserted after every 4 consecutive identical bits.
6. Mode message: GF(16) RS-protected (7 nibbles compact, 10 nibbles full).
7. Grid: reference grid (full only) → bullseye → orientation marks → mode message.
8. Data bits placed in clockwise layer spiral, inside → outside.

## Dependencies

- `barcode-2d` — `ModuleGrid` type and `Layout()` function
- `paint-instructions` — `PaintScene` type for downstream rendering

## Package matrix

| Language | Directory |
|----------|-----------|
| Go | `code/packages/go/aztec-code/` ← this package |
| TypeScript | `code/packages/typescript/aztec-code/` |
| Rust | `code/packages/rust/aztec-code/` |
| Python | `code/packages/python/aztec-code/` |
| Ruby | `code/packages/ruby/aztec_code/` |

## Testing

```
go test ./... -v -cover
```

## Version

0.1.0 — initial release (byte-mode only, auto-select compact/full).
