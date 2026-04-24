# com.codingadventures:paint-instructions

Backend-neutral 2D paint intermediate representation for Java.

## What is this?

`paint-instructions` defines the lightweight IR (intermediate representation)
that sits between high-level drawing abstractions (barcode layout engines,
chart builders) and concrete rendering backends (SVG, Canvas, Metal, terminal).

```
QR encoder  ─┐
DataMatrix  ─┼──→  PaintScene  ──→  SVG backend
MaxiCode    ─┘                  ──→  Canvas backend
                                 ──→  Terminal backend
```

Without this IR, every encoder would need direct knowledge of every backend:
N encoders × M backends combinations. With the IR we only need N + M adapters.

## Key types

### PathCommand (sealed)

A single drawing command inside a vector path — like instructions to a pen plotter:

```java
PathCommand move  = new PathCommand.MoveTo(10.0, 20.0);   // lift pen, move
PathCommand line  = new PathCommand.LineTo(50.0, 20.0);   // draw line
PathCommand close = PathCommand.ClosePath.INSTANCE;        // close shape
```

The sealed hierarchy means the compiler enforces exhaustive dispatch — add a new
subtype and every `instanceof` chain that needs updating gets flagged.

### PaintInstruction (sealed)

Two subtypes cover all current 2D barcode shapes:

- **PaintRect** — filled axis-aligned rectangle. Used by QR Code, Data Matrix,
  Aztec Code, PDF417.
- **PaintPath** — filled closed polygon built from `PathCommand`s. Used by
  MaxiCode (flat-top hexagons).

```java
PaintInstruction rect = new PaintInstruction.PaintRect(40, 40, 10, 10, "#000000");
PaintInstruction hex  = new PaintInstruction.PaintPath(hexCommands, "#000000");
```

### PaintScene

Top-level container passed to a paint backend:

```java
PaintScene scene = new PaintScene(210, 210, "#ffffff", List.of(rect, hex));
// scene.width  = 210
// scene.height = 210
// scene.background = "#ffffff"
// scene.instructions = [rect, hex]
```

### PaintInstructions (utility class)

Static builder helpers with sensible defaults:

```java
import static com.codingadventures.paintinstructions.PaintInstructions.*;

PaintScene scene = createScene(210, 210, "#ffffff", List.of(
    paintRect(0, 0, 210, 210, "#ffffff"),   // background
    paintRect(40, 40, 10, 10, "#000000"),   // dark module
    paintPath(hexCommands, "#000000")       // hex module
));
```

## Where this fits in the stack

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid                    ← produced by encoder
  → barcode-2d layout()           ← converts to pixels
  → PaintScene                    ← THIS PACKAGE
  → paint backend (SVG, Metal…)   ← renders to screen
```

## Requirements

- Java 21 (sealed classes introduced in Java 17, stable in 21)
- No runtime dependencies

## Building

```sh
mise exec -- gradle test
```
