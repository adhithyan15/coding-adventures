# micro-qr (Java)

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

## What it does

Encodes a string into a Micro QR Code symbol and returns a `ModuleGrid` (a
2D boolean grid of dark/light modules) ready for rendering.  It does not
decode Micro QR; that is a separate, more complex problem.

Micro QR Code is the compact variant of standard QR Code, designed for
applications where even the smallest regular QR (21×21 at version 1) is too
large.  Common use cases: surface-mount component labels, circuit board
markings, and miniature industrial tags.

## How it fits in the stack

```
paint-metal / paint-vm-svg / paint-vm-canvas
          └── paint-vm
                 └── paint-instructions
                          └── barcode-2d   ←── micro-qr ──→ gf256 (MA01)
```

`micro-qr` sits between the data representation layer (`barcode-2d`) and the
application layer.  It receives a plain string and hands back a `ModuleGrid`
that any `barcode-2d` renderer can display.

## Symbol sizes

| Symbol | Modules | Numeric | Alphanumeric | Byte |
|--------|---------|---------|--------------|------|
| M1     | 11×11   | 5       | —            | —    |
| M2     | 13×13   | 10 (L)  | 6 (L)        | 4 (L)|
| M3     | 15×15   | 23 (L)  | 14 (L)       | 9 (L)|
| M4     | 17×17   | 35 (L)  | 21 (L)       | 15(L)|

## Quick start

```java
import com.codingadventures.microqr.MicroQR;
import com.codingadventures.microqr.MicroQR.MicroQRVersion;
import com.codingadventures.microqr.MicroQR.EccLevel;
import com.codingadventures.barcode2d.ModuleGrid;

// Auto-select smallest symbol and ECC level:
ModuleGrid grid = MicroQR.encode("HELLO");
// grid.rows == 13 (M2 symbol)

// Specify version and ECC:
ModuleGrid m4 = MicroQR.encode("https://a.b", MicroQRVersion.M4, EccLevel.L);
// m4.rows == 17

// Render to SVG (requires paint-vm-svg):
// String svg = PaintVmSvg.paint(MicroQR.layout(grid));
```

## API

### `MicroQR.encode(String input)`
Auto-selects the smallest symbol and most compact mode.  Equivalent to
`encode(input, null, null)`.

### `MicroQR.encode(String input, MicroQRVersion version, EccLevel ecc)`
Encode to a specific version and/or ECC level.  Pass `null` for either to
use auto-selection.

**Throws:**
- `InputTooLongException` — input exceeds M4 capacity.
- `ECCNotAvailableException` — version+ECC combination not valid.
- `UnsupportedModeException` — no encoding mode covers the input.

### `MicroQRVersion`
`M1`, `M2`, `M3`, `M4`

### `EccLevel`
`DETECTION` (M1 only), `L`, `M`, `Q` (Q = M4 only).  Level `H` is not
available in any Micro QR symbol.

## Key differences from regular QR Code

| Feature | Micro QR | Regular QR |
|---------|----------|------------|
| Finder patterns | 1 (top-left) | 3 |
| Timing row/col | Row 0 / Col 0 | Row 6 / Col 6 |
| Mask patterns | 4 | 8 |
| Format XOR mask | 0x4445 | 0x5412 |
| Format info copies | 1 | 2 |
| Quiet zone | 2 modules | 4 modules |
| Mode indicator | 0–3 bits | 4 bits |
| Block interleaving | None (single block) | Yes |

## Dependencies

- `com.codingadventures:gf256` — GF(256) field arithmetic for Reed-Solomon ECC.
- `com.codingadventures:barcode-2d` — `ModuleGrid` type and layout function.

## Build

```bash
./gradlew test
```

Requires Java 21.  Uses Gradle 8.x with composite builds for local dependencies.
