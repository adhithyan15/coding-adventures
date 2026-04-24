# coding_adventures_barcode_2d

Shared 2D barcode abstraction layer for the coding-adventures monorepo.

## What this package does

This package sits between the format-specific encoders (QR Code, Data Matrix,
Aztec, MaxiCode, …) and the pixel-level paint backend. It defines two things:

1. **`ModuleGrid`** — the universal immutable 2D boolean grid that every encoder
   produces. `true` = dark module (ink); `false` = light module (background).
2. **`layout()`** — the **only** function in the entire barcode stack that knows
   about pixels. It reads a `Barcode2DLayoutConfig`, multiplies module
   coordinates by `moduleSizePx`, and emits a `PaintScene`.

Callers never need to think about pixels until `layout()` is called.

## Where it fits in the pipeline

```
Input data (text, bytes, URL, …)
  → format encoder (qr-code, data-matrix, aztec, maxicode…)
  → ModuleGrid          ← THIS PACKAGE defines the grid model
  → layout()            ← THIS PACKAGE converts ModuleGrid → PaintScene
  → PaintScene          ← defined in coding_adventures_paint_instructions
  → paint-vm backend    ← renders PaintScene to SVG, Canvas, PNG, …
```

## Types

| Type                          | Description                                                  |
|-------------------------------|--------------------------------------------------------------|
| `ModuleShape`                 | Enum: `square` or `hex`                                      |
| `ModuleRole`                  | Enum: structural role for visualizer colour-coding           |
| `ModuleAnnotation`            | Per-module role annotation (ignored by `layout()`)           |
| `ModuleGrid`                  | Immutable 2D boolean grid                                    |
| `AnnotatedModuleGrid`         | `ModuleGrid` + per-module `ModuleAnnotation?` list           |
| `Barcode2DLayoutConfig`       | Layout parameters (size, quiet zone, colours, shape)         |
| `Barcode2DError`              | Base exception class                                         |
| `InvalidBarcode2DConfigError` | Thrown for bad config or grid/config shape mismatch          |
| `PaintScene`                  | Re-exported from `coding_adventures_paint_instructions`      |

## Usage

### Basic QR Code layout

```dart
import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

// 1. Create an all-light 21×21 grid (QR Code v1 size).
ModuleGrid grid = makeModuleGrid(rows: 21, cols: 21);

// 2. Encoders set dark modules one at a time (purely functional — each call
//    returns a new grid; the original is unchanged).
grid = setModule(grid, row: 0, col: 0, dark: true);
grid = setModule(grid, row: 0, col: 20, dark: true);
// …continue placing finder patterns, timing strips, data bits, ECC bits…

// 3. Convert to paint instructions using the default config:
//    moduleSizePx=10, quietZoneModules=4, black on white, square modules.
final PaintScene scene = layout(grid);
// scene.width  == 290  ((21 + 2*4 quiet-zone modules) * 10 px)
// scene.height == 290
```

### Custom layout config

```dart
final cfg = defaultBarcode2DLayoutConfig.copyWith(
  moduleSizePx: 4,         // smaller modules for a compact barcode
  quietZoneModules: 1,     // Data Matrix only requires 1-module quiet zone
  foreground: '#1a1a1a',   // near-black instead of pure black
  background: '#f8f8f0',   // off-white background
);

final scene = layout(grid, config: cfg);
```

### layoutSquare — square modules (QR, Data Matrix, Aztec, PDF417)

Most 2D barcode formats use rectangular modules. Pass a `ModuleGrid` created
with the default `ModuleShape.square` (or omit the `moduleShape` argument):

```dart
// Default shape is square — no need to specify.
final grid = makeModuleGrid(rows: 21, cols: 21);
final scene = layout(grid);                    // uses square rendering path
```

Each dark module becomes one `PaintRect`. The scene always starts with a single
background `PaintRect` covering the entire canvas (including the quiet zone).

### layoutHex — hex modules (MaxiCode / ISO/IEC 16023)

MaxiCode uses flat-top hexagons in an offset-row tiling. Create the grid with
`ModuleShape.hex` and pass a matching config:

```dart
// MaxiCode grids are always 33 rows × 30 columns.
ModuleGrid grid = makeModuleGrid(rows: 33, cols: 30, moduleShape: ModuleShape.hex);

// … set dark modules …

final cfg = defaultBarcode2DLayoutConfig.copyWith(
  moduleShape: ModuleShape.hex,
  quietZoneModules: 1,
);
final scene = layout(grid, config: cfg);
```

Each dark module becomes one `PaintPath` with seven commands:
`MoveTo` + five `LineTo` + `Close`, tracing a flat-top regular hexagon. Odd
rows shift right by half a module width to produce the standard hex tiling.

### ModuleAnnotation — visualizer colour-coding

Annotations are optional metadata that visualizers can use to colour-code the
structural regions of a barcode. They are **never** read by `layout()`.

```dart
const ann = ModuleAnnotation(
  role: ModuleRole.finder,
  dark: true,
  metadata: {'format_role': 'qr:finder-top-left'},
);
```

Build an `AnnotatedModuleGrid` when you want to pass both the grid and its
annotations downstream:

```dart
final annotated = AnnotatedModuleGrid(
  rows: grid.rows,
  cols: grid.cols,
  modules: grid.modules,
  moduleShape: grid.moduleShape,
  annotations: myAnnotationTable,   // List<List<ModuleAnnotation?>>
);

// layout() accepts AnnotatedModuleGrid exactly like ModuleGrid.
final scene = layout(annotated);
```

## Barcode2DLayoutConfig reference

| Field              | Default     | Constraint      | Notes                                                           |
|--------------------|-------------|-----------------|------------------------------------------------------------------|
| `moduleSizePx`     | `10`        | `> 0`           | Side length of one module in pixels                              |
| `quietZoneModules` | `4`         | `>= 0`          | QR requires ≥ 4; Data Matrix ≥ 1; MaxiCode ≥ 1                 |
| `foreground`       | `"#000000"` | CSS hex string  | Fill colour for dark modules                                     |
| `background`       | `"#ffffff"` | CSS hex string  | Fill colour for background + light modules                       |
| `moduleShape`      | `square`    | must match grid | `layout()` throws if this disagrees with `grid.moduleShape`     |

Use `copyWith()` to override individual fields without repeating the rest:

```dart
final cfg = defaultBarcode2DLayoutConfig.copyWith(moduleSizePx: 20);
```

## Error handling

`layout()` throws `InvalidBarcode2DConfigError` (a subclass of `Barcode2DError`)
when the config is invalid:

```dart
try {
  final scene = layout(grid, config: cfg);
} on InvalidBarcode2DConfigError catch (e) {
  print(e.message); // human-readable explanation
}
```

Common causes:
- `moduleSizePx <= 0` — must be a positive integer
- `quietZoneModules < 0` — must be non-negative
- `config.moduleShape != grid.moduleShape` — grid was created with hex shape but
  config says square (or vice versa)

## Immutability and the encoder pattern

`ModuleGrid` is intentionally immutable. Encoders call `makeModuleGrid()` once
to get an all-light grid, then repeatedly call `setModule()` to paint dark
modules one at a time:

```dart
// Each setModule() returns a new grid; the original is untouched.
ModuleGrid g = makeModuleGrid(rows: 5, cols: 5);
g = setModule(g, row: 0, col: 0, dark: true);
g = setModule(g, row: 0, col: 1, dark: true);
```

Only the modified row is re-allocated; all unchanged rows are structurally
shared between old and new grids. Memory usage is proportional to the number
of updates, not total grid area.

This makes encoders easy to test (take a snapshot of the grid at any step) and
supports backtracking without an undo stack — for example, QR Code mask
evaluation tries all eight mask patterns and keeps the best-scoring one by
simply saving the reference before masking.

## Running tests

```sh
cd code/packages/dart/barcode-2d
dart pub get
dart test
```

## Design decisions

- **Single layout function** — `layout()` is the only pixel-aware function in the
  entire barcode stack. Everything above works in abstract module units. This
  separation makes format encoders easy to unit-test without needing a pixel
  renderer.
- **Shape dispatch in layout** — `ModuleShape` is stored on the grid so `layout()`
  can pick the right rendering path automatically. The shape mismatch check
  prevents accidentally rendering a MaxiCode hex grid with square modules.
- **Re-exported PaintScene** — callers can type the `layout()` return value
  without importing `coding_adventures_paint_instructions` directly.
- **Immutable grids** — see the section above. Backtracking support for QR mask
  evaluation is the main motivator.
- **Annotations separate from modules** — `ModuleAnnotation` lives in
  `AnnotatedModuleGrid`, not in the base `ModuleGrid`. This keeps the hot path
  (encoder → layout → paint) free of visualizer overhead.
