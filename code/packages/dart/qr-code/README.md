# coding_adventures_qr_code

A QR Code encoder for Dart, implementing ISO/IEC 18004:2015.

Encodes any UTF-8 string into a scannable QR Code module grid. Integrates with
the `coding_adventures_barcode_2d` layout pipeline for pixel rendering.

## Features

- **Modes**: numeric, alphanumeric (45-char set), byte (UTF-8)
- **ECC levels**: L (~7%), M (~15%), Q (~25%), H (~30%)
- **Versions 1–40**: automatic version selection (smallest that fits)
- **Full pipeline**: RS ECC (GF(256) b=0), block interleaving, mask evaluation
- **Paint pipeline integration**: `encodeAndLayout()` → `PaintScene`
- **Literate source**: Knuth-style comments explain every algorithm step

## Quick Start

```dart
import 'package:coding_adventures_qr_code/coding_adventures_qr_code.dart';

// Encode a string into a module grid.
final grid = encode('HELLO WORLD', EccLevel.m);
print('${grid.rows}×${grid.cols}'); // 21×21 (version 1)

// Encode and produce pixel-level paint instructions.
final scene = encodeAndLayout('https://example.com', EccLevel.m);
print('${scene.width}×${scene.height}'); // 250×250 px (v2, 10px modules, 4-module quiet zone)

// Custom rendering config.
const cfg = Barcode2DLayoutConfig(
  moduleSizePx: 4,
  quietZoneModules: 4,
  foreground: '#000000',
  background: '#ffffff',
  moduleShape: ModuleShape.square,
);
final scene2 = encodeAndLayout('HELLO WORLD', EccLevel.h, config: cfg);
```

## Error Handling

```dart
try {
  final grid = encode(veryLongString, EccLevel.h);
} on InputTooLongError catch (e) {
  print('Too long: ${e.message}');
}
```

## ECC Level Guide

| Level | Recovery | Use when |
|-------|----------|----------|
| `EccLevel.l` | ~7% | Large, clean symbols |
| `EccLevel.m` | ~15% | General purpose (default) |
| `EccLevel.q` | ~25% | Labels with partial damage expected |
| `EccLevel.h` | ~30% | Harsh environments, logos over QR |

## Capacity (byte mode, rough guide)

| Version | Size | ECC L | ECC M | ECC Q | ECC H |
|---------|------|-------|-------|-------|-------|
| 1       | 21×21 | 17 B | 14 B | 11 B | 7 B |
| 5       | 37×37 | 106 B | 84 B | 60 B | 44 B |
| 10      | 57×57 | 271 B | 213 B | 151 B | 106 B |
| 20      | 97×97 | 858 B | 666 B | 482 B | 342 B |
| 40      | 177×177 | 2953 B | 2331 B | 1663 B | 1273 B |

## Dependency Stack

```
barcode-2d  ←── paint-instructions
    ↑
qr-code ──── gf256
```

`qr-code` depends on `gf256` for GF(256) arithmetic in the Reed-Solomon
encoder, and on `barcode-2d` for the `ModuleGrid` type and the `layout()`
function.

## API Reference

### `encode(String input, EccLevel ecc) → ModuleGrid`

Encodes `input` into a QR Code module grid. Throws `InputTooLongError` if the
input is too long for any version at the given ECC level.

### `encodeAndLayout(String input, EccLevel ecc, {Barcode2DLayoutConfig? config}) → PaintScene`

Convenience: encode then layout in one call. Throws `InputTooLongError` or
`QRLayoutError`.

### `EccLevel`

Enum with values: `l`, `m`, `q`, `h`.

### `InputTooLongError`

Thrown when the input exceeds version-40 capacity at the chosen ECC level.

### `QRLayoutError`

Thrown when the `Barcode2DLayoutConfig` is invalid (e.g. `moduleSizePx <= 0`).

## Design Notes

### Format information bit ordering

The format information is placed MSB-first in row 8 (f14→f9 for cols 0–5),
following the correction documented in `lessons.md` (2026-04-23). This is the
ordering that real QR scanners expect. Using the wrong ordering produces symbols
that appear visually correct but are rejected by every standard scanner.

### Reed-Solomon convention

QR Code uses the **b=0 convention**: generator polynomial roots are
α^0, α^1, …, α^{n-1}. This package builds the generator on demand from the
`gf256` package rather than embedding precomputed tables, making the RS
implementation transparent and verifiable.

### UTF-8 in byte mode

Byte mode encodes raw UTF-8 bytes. Modern QR scanners default to UTF-8.
For ASCII strings, byte mode and UTF-8 are identical.

## License

Part of the `coding-adventures` monorepo.
