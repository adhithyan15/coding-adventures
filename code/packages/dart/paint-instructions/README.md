# coding_adventures_paint_instructions

Backend-neutral paint scene model for the coding-adventures monorepo.

## What this package does

This package defines the tiny intermediate representation that sits between
abstract data (barcode grids, vector graphics) and concrete pixel backends
(SVG, Canvas 2D, Metal, Direct2D, terminal ASCII, …).

Rather than hard-coding "draw SVG rectangles" inside a QR encoder, encoders
produce a `PaintScene` — a structured list of fill instructions — and let a
separate backend turn that into pixels.

## Where it fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → barcode-2d layout() ← converts ModuleGrid → PaintScene
  → PaintScene          ← THIS PACKAGE defines the model
  → paint-vm backend    ← renders PaintScene to pixels
  → output (SVG, PNG, terminal, …)
```

## Types

| Type               | Description                                              |
|--------------------|----------------------------------------------------------|
| `PathCommand`      | Sealed: `MoveTo`, `LineTo`, `Close`                      |
| `PaintInstruction` | Sealed base: `PaintRect`, `PaintPath`                    |
| `PaintRect`        | Axis-aligned filled rectangle                            |
| `PaintPath`        | Filled polygon described by `PathCommand` list           |
| `PaintScene`       | Complete frame: width, height, background, instructions  |
| `PaintColorRGBA8`  | Parsed RGBA colour (one byte per channel)                |

## Usage

```dart
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

// Build a 100×100 scene with a red square in the centre.
final scene = createScene(
  width: 100,
  height: 100,
  background: '#ffffff',
  instructions: [
    paintRect(x: 25, y: 25, width: 50, height: 50, fill: '#ff0000'),
  ],
);

// Parse a color from a CSS hex string.
final color = parseColorRGBA8('#ff000080'); // semi-transparent red
print(color.a); // 128

// Build a hexagonal path.
final hex = paintPath(
  commands: [
    PathCommand.moveTo(10, 0),
    PathCommand.lineTo(20, 17),
    PathCommand.lineTo(10, 34),
    PathCommand.lineTo(0, 17),
    PathCommand.close(),
  ],
  fill: '#000000',
);
```

## Running tests

```sh
dart pub get
dart test
```

## Design decisions

- **Sealed classes** — both `PathCommand` and `PaintInstruction` are sealed so
  the compiler enforces exhaustive switch expressions. Adding a new instruction
  kind requires touching every switch in the codebase — intentional.
- **Immutable values** — all types use `final` fields. Instructions and scenes
  are safe to share across threads (Dart isolates).
- **Sane defaults** — helper functions (`paintRect`, `paintPath`, `createScene`)
  default fill to `#000000` and background to `#ffffff`, matching the common
  "black ink on white paper" case.
- **String colours** — fill colours are CSS hex strings rather than structs.
  This keeps the model backend-neutral: SVG backends can use the string
  directly; native backends call `parseColorRGBA8`.
