# paint-instructions (Kotlin)

Backend-neutral 2D paint scene model for the coding-adventures monorepo.

## What this package does

`paint-instructions` defines a lightweight intermediate representation (IR) that
sits between a high-level drawing producer (a barcode encoder, a diagram
renderer, etc.) and a concrete output backend (SVG, Canvas 2D, Metal, terminal).

Without an IR every producer needs to know how to target every backend — an N×M
explosion of combinations.  With an IR we only need N producer → IR adapters and
M IR → backend adapters.

```
QR encoder  ─┐
DataMatrix  ─┼──→  PaintScene  ──→  SVG backend
MaxiCode    ─┘                  ──→  Canvas backend
                                 ──→  Terminal backend
```

## Core types

| Type | Description |
|------|-------------|
| `PaintScene` | Canvas dimensions + background colour + ordered instruction list |
| `PaintInstruction.PaintRect` | Filled axis-aligned rectangle |
| `PaintInstruction.PaintPath` | Filled closed polygon (for hex modules) |
| `PathCommand` | `MoveTo`, `LineTo`, `ClosePath` drawing commands |
| `PaintColorRGBA8` | 32-bit RGBA colour |
| `Metadata` | `Map<String, String>` annotation bag |

## Helper functions

```kotlin
// Build a filled rectangle:
val rect = paintRect(x = 10, y = 20, width = 10, height = 10, fill = "#000000")

// Build a filled polygon:
val hex = paintPath(commands = hexCommands, fill = "#1a1a1a")

// Build a complete scene:
val scene = createScene(width = 210, height = 210, instructions = listOf(rect, hex))

// Parse a CSS colour string:
val black = parseColorRGBA8("#000000")   // PaintColorRGBA8(0, 0, 0, 255)
val red   = parseColorRGBA8("#f00")      // PaintColorRGBA8(255, 0, 0, 255)
```

## Where this fits in the stack

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → barcode-2d layout() ← converts to pixels via paint-instructions
  → PaintScene          ← THIS PACKAGE's output type
  → paint-vm backend    ← renders to SVG / Canvas / Metal / terminal
```

## Relationship to the Go reference

This Kotlin package mirrors `code/packages/go/paint-instructions/` semantically.
Go uses interfaces and structs; Kotlin uses sealed classes and data classes.
The public API names (`paintRect`, `paintPath`, `createScene`,
`parseColorRGBA8`) are kept consistent across both languages.

## Building and testing

```bash
# Run tests (from the repo root via mise):
mise exec -- bash -c "cd code/packages/kotlin/paint-instructions && ./gradlew test --no-daemon"
```

Requires JDK 21 and Gradle (downloaded automatically by the wrapper).

## Package info

- **Group:** `com.codingadventures`
- **Artifact:** `paint-instructions`
- **Version:** `0.1.0`
- **Language:** Kotlin 2.1.20, JVM target 21
- **Test framework:** JUnit Jupiter 5.11.4
