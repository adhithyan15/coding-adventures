# com.codingadventures:barcode-2d

Shared 2D barcode abstraction layer for Java.

## What is this?

`barcode-2d` provides the two building blocks that every 2D barcode encoder needs:

1. **`ModuleGrid`** — the universal intermediate representation produced by every
   2D barcode encoder (QR Code, Data Matrix, Aztec Code, PDF417, MaxiCode). It is
   a 2D boolean grid: `true` = dark module (ink), `false` = light module (background).

2. **`Barcode2D.layout()`** — the single function that converts abstract module
   coordinates into pixel-level `PaintScene` instructions ready for a paint backend.

## Where this fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder (this package)
  → layout()            ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by paint-vm
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout()` are measured in "module units" — abstract grid steps.
Only `layout()` multiplies by `moduleSizePx` to produce real pixel coordinates. This means
encoders never need to know anything about screen resolution or output format.

## Supported module shapes

### Square (SQUARE) — QR Code, Data Matrix, Aztec Code, PDF417

Each dark module becomes a `PaintInstruction.PaintRect`:

```java
ModuleGrid grid = Barcode2D.makeModuleGrid(21, 21);
grid = Barcode2D.setModule(grid, 0, 0, true);  // top-left dark
// ...
PaintScene scene = Barcode2D.layout(grid);
// scene.width  = (21 + 2*4) * 10 = 290 px
// scene.height = 290 px
// scene.instructions = [background_rect, dark_rect_0_0, ...]
```

### Hex (HEX) — MaxiCode

MaxiCode uses flat-top hexagons in an offset-row grid. Odd rows shift right
by half a hexagon width to produce the standard hexagonal tiling:

```
Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hexWidth/2)
Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
```

Each dark hex module becomes a `PaintInstruction.PaintPath` with 7 commands
(1 MoveTo, 5 LineTo, 1 ClosePath):

```java
ModuleGrid grid = Barcode2D.makeModuleGrid(33, 30, ModuleShape.HEX);
grid = Barcode2D.setModule(grid, 16, 15, true);  // centre module
Barcode2DLayoutConfig config = new Barcode2DLayoutConfig.Builder()
    .moduleShape(ModuleShape.HEX)
    .moduleSizePx(10)
    .build();
PaintScene scene = Barcode2D.layout(grid, config);
```

## Key APIs

### makeModuleGrid

```java
ModuleGrid grid = Barcode2D.makeModuleGrid(21, 21);            // square
ModuleGrid hex  = Barcode2D.makeModuleGrid(33, 30, ModuleShape.HEX); // hex
```

### setModule (immutable copy-on-write)

```java
ModuleGrid g2 = Barcode2D.setModule(grid, row, col, true);
// grid is unchanged; g2 has one dark module at (row, col)
```

### layout with config

```java
Barcode2DLayoutConfig config = new Barcode2DLayoutConfig.Builder()
    .moduleSizePx(5)          // smaller modules
    .quietZoneModules(1)      // minimal quiet zone
    .foreground("#1a1a1a")
    .background("#f5f5f5")
    .build();

PaintScene scene = Barcode2D.layout(grid, config);
```

## Dependencies

- `com.codingadventures:paint-instructions` (local composite build)

## Requirements

- Java 21

## Building

```sh
mise exec -- gradle test
```
