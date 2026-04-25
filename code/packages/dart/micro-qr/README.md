# coding_adventures_micro_qr

Micro QR Code encoder for Dart — ISO/IEC 18004:2015 Annex E compliant.

Encodes strings to Micro QR Code symbols (M1–M4, 11×11 to 17×17 modules).
The encoder produces a `ModuleGrid` that can be rendered to SVG, canvas, or
any pixel target via the `barcode-2d` layout layer.

## What is Micro QR?

Micro QR Code is the compact sibling of regular QR Code. It was designed for
applications where even the smallest standard QR (21×21 at version 1) is too
large. Common uses include surface-mount component labels, circuit board
markings, and miniature industrial tags.

```
M1: 11×11 modules   M2: 13×13 modules
M3: 15×15 modules   M4: 17×17 modules
formula: size = 2 × version_number + 9
```

## Key differences from regular QR Code

| Feature                    | Regular QR        | Micro QR         |
|----------------------------|-------------------|------------------|
| Finder patterns            | 3 (three corners) | 1 (top-left only)|
| Timing pattern location    | Row 6 / col 6     | Row 0 / col 0    |
| Mask patterns              | 8                 | 4                |
| Format info copies         | 2                 | 1                |
| Format XOR mask            | 0x5412            | 0x4445           |
| Quiet zone                 | 4 modules         | 2 modules        |
| Mode indicator width       | 4 bits            | 0–3 bits         |
| Block structure            | Multi-block       | Single block     |
| Max capacity               | 7,089 numeric     | 35 numeric       |

## Quick start

```dart
import 'package:coding_adventures_micro_qr/coding_adventures_micro_qr.dart';

// Auto-select smallest symbol for "HELLO" → M2-L (13×13)
final grid = encode('HELLO');
print(grid.rows);  // 13

// Force M4-L (17×17) with low ECC for maximum capacity
final big = encode('https://example.com',
    version: MicroQRVersion.m4, ecc: MicroQREccLevel.l);
print(big.rows);  // 17

// Encode and lay out in one call
final scene = encodeAndLayout('12345');

// Encode to a specific version/ECC combination
final explicit = encodeAt('HELLO', MicroQRVersion.m4, MicroQREccLevel.q);
```

## Encoding modes

| Mode          | Available in  | Character set                         | Bits/char |
|---------------|--------------|---------------------------------------|-----------|
| Numeric       | M1–M4        | Digits 0–9                            | ~3.3      |
| Alphanumeric  | M2–M4        | 0–9, A–Z, space, $%*+-./: (45 chars) | ~5.5      |
| Byte          | M3–M4        | Any bytes (UTF-8 encoded)             | 8.0       |
| Kanji         | M4 only      | Shift-JIS (future extension)          | 13.0      |

## Capacity table

| Symbol | ECC | Numeric | Alphanumeric | Byte |
|--------|-----|---------|--------------|------|
| M1     | Det | 5       | —            | —    |
| M2     | L   | 10      | 6            | 4    |
| M2     | M   | 8       | 5            | 3    |
| M3     | L   | 23      | 14           | 9    |
| M3     | M   | 18      | 11           | 7    |
| M4     | L   | 35      | 21           | 15   |
| M4     | M   | 30      | 18           | 13   |
| M4     | Q   | 21      | 13           | 9    |

## ECC levels

| Level      | Availability | Recovery |
|------------|-------------|---------|
| `detection`| M1 only     | Detects errors, cannot correct |
| `l`        | M2, M3, M4  | ~7% of codewords recoverable |
| `m`        | M2, M3, M4  | ~15% of codewords recoverable |
| `q`        | M4 only     | ~25% of codewords recoverable |

Level H (high, 30%) is not available in any Micro QR symbol.

## Error handling

```dart
try {
  final grid = encode('1' * 36); // too long for any M1–M4
} on InputTooLong catch (e) {
  print(e.message); // "...Maximum is 35 numeric chars in M4-L..."
}

try {
  encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.l);
} on ECCNotAvailable catch (e) {
  print(e.message); // "...M1 only supports detection..."
}
```

All encoder errors extend `MicroQRError` (implements `Exception`).

## Package position in the stack

```
Input string
  → encode() / encodeAt()       ← THIS PACKAGE
  → ModuleGrid
  → layoutGrid() / layout()     ← barcode-2d
  → PaintScene
  → paint-vm-svg / paint-metal  ← rendering backend
  → SVG / PNG / native window
```

This package depends on:
- `coding_adventures_barcode_2d` — `ModuleGrid`, `layout()`, `PaintScene`
- `coding_adventures_gf256` — `gfMultiply()` for Reed-Solomon ECC

This package does **not** depend on `coding_adventures_qr_code` — Micro QR
and regular QR are siblings, not parent/child.

## Running tests

```sh
dart pub get
dart test -r expanded
```
