# coding_adventures_aztec_code

Aztec Code 2D barcode encoder conforming to ISO/IEC 24778:2008, written in
pure Dart.

## What is Aztec Code?

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
released into the public domain.  Unlike QR Code — which places three square
finder patterns at three corners — Aztec Code has a single **bullseye finder
pattern at the centre** of the symbol.  Scanners locate the bullseye first, then
read outward in a clockwise spiral.  No large quiet zone is required.

### Where Aztec Code is used today

| Application | Detail |
|---|---|
| IATA boarding passes | The barcode on every airline boarding pass |
| Eurostar / Amtrak tickets | Printed and on-screen rail tickets |
| European postal routing | PostNL, Deutsche Post, La Poste |
| US military ID cards | DoD Common Access Card |

## Symbol variants

```
Compact: 1–4 layers,  size = 11 + 4×layers   (15×15 to 27×27 modules)
Full:    1–32 layers, size = 15 + 4×layers   (19×19 to 143×143 modules)
```

The encoder automatically selects the smallest symbol that satisfies the
requested ECC percentage (default 23%).

## Quick start

```dart
import 'package:coding_adventures_aztec_code/coding_adventures_aztec_code.dart';

// Encode a string → ModuleGrid.
final grid = encode('IATA BP DATA');
print(grid.rows);   // e.g. 19 (compact-2)
print(grid.cols);   // 19 (always square)

// Encode + layout → PaintScene (ready for an SVG or canvas backend).
final scene = encodeAndLayout('Hello, World!');
print(scene.width);  // pixels

// Increase error correction to 33%.
final grid33 = encode('FRAGILE DATA', options: const AztecOptions(minEccPercent: 33));
```

## Public API

### `encode(String data, {AztecOptions options}) → ModuleGrid`

Encodes [data] (as UTF-8 code units) into the smallest Aztec symbol that fits
at the requested ECC level.  Returns a [ModuleGrid] where
`grid.modules[row][col]` is `true` for a dark module.

Throws [InputTooLongError] if the data exceeds the maximum symbol capacity
(~1914 bytes at 23% ECC in a 32-layer full symbol).

### `encodeAndLayout(String data, {AztecOptions?, Barcode2DLayoutConfig?}) → PaintScene`

Convenience wrapper: encode then layout in one call.

### `layoutGrid(ModuleGrid grid, {Barcode2DLayoutConfig?}) → PaintScene`

Convert an already-encoded [ModuleGrid] to pixel-level paint instructions.

### `explain(String data, {AztecOptions?}) → AnnotatedModuleGrid`

Encode and return an [AnnotatedModuleGrid].  In v0.1.0 per-module annotations
are not populated (stub returning null for every cell).  Full annotation support
is planned for v0.2.0.

### `AztecOptions`

```dart
class AztecOptions {
  final int minEccPercent;  // default: 23, range: 10–90
  const AztecOptions({this.minEccPercent = 23});
}
```

### Error types

| Type | When thrown |
|---|---|
| `AztecError` | Base class; catch this to handle any encoder error |
| `InputTooLongError` | Payload exceeds 32-layer full symbol capacity |

### Constants

| Symbol | Value |
|---|---|
| `aztecCodeVersion` | `'0.1.0'` |

## Encoding pipeline

```
input string (UTF-8 code units)
  1. → Binary-Shift from Upper mode
       5-bit escape (11111) + length (5 or 16 bits) + 8-bit bytes
  2. → smallest symbol at minEccPercent
       compact 1→4, then full 1→32
  3. → pad data to exact codeword count
       zero-pad to byte boundary, then append zeroes
  4. → GF(256)/0x12D Reed-Solomon ECC
       b=1 convention (roots α^1..α^n), same polynomial as Data Matrix
  5. → bit stuffing
       insert complement bit after every run of 4 identical bits
  6. → GF(16) mode message
       28 bits (compact) or 40 bits (full), 5 or 6 RS nibbles
  7. → grid construction
       reference grid (full only) → bullseye → orientation marks →
       mode message → data spiral
  8. → ModuleGrid
```

## v0.1.0 limitations

- **Byte mode only** — all input encoded via Binary-Shift from Upper mode.
  Multi-mode optimisation (Digit / Upper / Lower / Mixed / Punct) is v0.2.0.
- **`explain()` annotations are stubs** — per-module role colouring is v0.2.0.

## Dependencies

| Package | Role |
|---|---|
| `coding_adventures_barcode_2d` | `ModuleGrid`, `layout()`, `PaintScene` |

## Running the tests

```sh
dart pub get
dart test -r expanded
```

## Package position in the stack

```
coding_adventures_aztec_code     ← this package
  └─ coding_adventures_barcode_2d  (ModuleGrid, layout)
       └─ coding_adventures_paint_instructions  (PaintScene, PaintRect)
```
