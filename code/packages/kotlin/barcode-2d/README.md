# barcode-2d (Kotlin)

Shared 2D barcode abstraction layer for the coding-adventures monorepo.

## What this package does

`barcode-2d` provides the two building blocks that every 2D barcode encoder
needs to produce a renderable output:

1. **`ModuleGrid`** — the universal intermediate representation produced by
   every 2D barcode encoder (QR Code, Data Matrix, Aztec Code, PDF417,
   MaxiCode).  It is a 2D boolean grid: `true` = dark module, `false` = light
   module.

2. **`layout()`** — the single function that converts abstract module
   coordinates into pixel-level `PaintScene` instructions ready for the
   PaintVM backend.

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → layout()            ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout()` are measured in "module units" — abstract
grid steps.  Only `layout()` multiplies by `Barcode2DLayoutConfig.moduleSizePx`
to produce real pixel coordinates.  This means encoders never need to know
anything about screen resolution or output format.

## Core types

| Type | Description |
|------|-------------|
| `ModuleGrid` | Immutable 2D boolean grid: `true` = dark, `false` = light |
| `ModuleShape` | `SQUARE` (default) or `HEX` (MaxiCode) |
| `ModuleRole` | Structural role: `FINDER`, `TIMING`, `DATA`, `ECC`, etc. |
| `ModuleAnnotation` | Per-module role metadata for visualizers |
| `AnnotatedModuleGrid` | `ModuleGrid` + per-module annotations |
| `Barcode2DLayoutConfig` | Pixel-level rendering options |
| `Barcode2DException` | Base exception class |
| `InvalidBarcode2DConfigException` | Invalid layout configuration |

## Core functions

| Function | Description |
|----------|-------------|
| `makeModuleGrid(rows, cols)` | Create an all-light grid |
| `setModule(grid, row, col, dark)` | Immutable single-module update |
| `layout(grid, config)` | Convert `ModuleGrid` → `PaintScene` |

## Usage example

```kotlin
import com.codingadventures.barcode2d.*

// 1. Create a 21×21 grid (QR Code version 1)
var grid = makeModuleGrid(rows = 21, cols = 21)

// 2. Paint some dark modules (encoder would do this systematically)
grid = setModule(grid, row = 0, col = 0, dark = true)
grid = setModule(grid, row = 0, col = 1, dark = true)

// 3. Convert to a PaintScene with default config (10 px/module, 4-module quiet zone)
val scene = layout(grid)
// scene.width == 290, scene.height == 290
// scene.instructions.size == 3 (background + 2 dark modules)

// 4. Custom config: 5 px modules, 1-module quiet zone, red-on-white
val config = Barcode2DLayoutConfig(
    moduleSizePx = 5,
    quietZoneModules = 1,
    foreground = "#cc0000",
    background = "#ffffff",
)
val smallScene = layout(grid, config)
// smallScene.width == 115, smallScene.height == 115
```

## MaxiCode (hex modules)

```kotlin
// MaxiCode uses flat-top hexagonal modules in a 33×30 grid
var hexGrid = makeModuleGrid(rows = 33, cols = 30, moduleShape = ModuleShape.HEX)
hexGrid = setModule(hexGrid, row = 1, col = 2, dark = true)

val hexConfig = Barcode2DLayoutConfig(moduleShape = ModuleShape.HEX)
val hexScene = layout(hexGrid, hexConfig)
// Each dark module → one PaintInstruction.PaintPath tracing a flat-top hexagon
```

## Supported module shapes

- **Square** (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
  Each dark module becomes one `PaintInstruction.PaintRect`.

- **Hex** (flat-top hexagons): used by MaxiCode (ISO/IEC 16023).
  Each dark module becomes one `PaintInstruction.PaintPath` with 6 vertices.
  Odd-numbered rows are offset right by half a hexagon width for standard
  hexagonal tiling.

## Immutability

`ModuleGrid` is intentionally immutable.  `setModule()` returns a new grid
with only the affected row re-allocated; all other rows are shared with the
original.  This makes encoders easy to test and compose without undo stacks
or defensive copies.

## Where this fits in the stack

```
DT2D01 barcode-2d  ← THIS PACKAGE
    ↑ uses
P2D01 paint-instructions  ← PaintScene, PaintRect, PaintPath

    ↓ used by
DT12 b-plus-tree, micro-qr, data-matrix, aztec …
```

## Relationship to the TypeScript reference

This package mirrors `code/packages/typescript/barcode-2d/src/index.ts`.
TypeScript uses interfaces and discriminated union types; Kotlin uses sealed
classes, data classes, and enum classes.  The public API names and semantics
are kept as consistent as the two type systems allow.

## Building and testing

```bash
# Run tests (from this directory via mise):
mise exec -- bash -c "cd code/packages/kotlin/barcode-2d && gradle test --no-daemon"
```

Requires JDK 21 and Gradle (downloaded automatically by the wrapper if present).

## Package info

- **Group:** `com.codingadventures`
- **Artifact:** `barcode-2d`
- **Version:** `0.1.0`
- **Language:** Kotlin 2.1.20, JVM target 21
- **Test framework:** JUnit Jupiter 5.11.4
- **Spec:** DT2D01
