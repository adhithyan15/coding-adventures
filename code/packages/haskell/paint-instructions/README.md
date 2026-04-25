# paint-instructions (Haskell)

Universal 2D paint intermediate representation (IR) — P2D00.

## What is this?

`paint-instructions` is the shared contract between **producers** (barcode
encoders, chart builders, diagram renderers) and **backends** (SVG, Canvas,
Metal, terminal) in the coding-adventures 2D painting pipeline.

```
Producer (chart, barcode, diagram)
  -> PaintScene / [PaintInstruction]   <- this package
  -> PaintVM (P2D01)
  -> Backend (SVG, Canvas, Metal, terminal)
```

Everything in this package is **pure data** — no IO, no side effects.
A `PaintScene` is a plain Haskell value.

## Core types

| Type | Description |
|------|-------------|
| `PathCommand` | One pen-plotter step: `MoveTo`, `LineTo`, `ClosePath` |
| `PaintInstruction` | A rect (`PaintRect`) or path (`PaintPath`) |
| `PaintScene` | Canvas dimensions + background + ordered instruction list |

## Usage

```haskell
import CodingAdventures.PaintInstructions
import qualified Data.Map.Strict as Map

-- A 200x100 white scene with one blue rectangle
example :: PaintScene
example = PaintScene
  { psWidth        = 200
  , psHeight       = 100
  , psBg           = "#ffffff"
  , psInstructions =
      [ makeRect 10 10 80 40 "#2563eb"
      ]
  , psMeta = Map.empty
  }

-- Build a triangle path
triangle :: PaintInstruction
triangle = makePath
  [ MoveTo 50 10, LineTo 90 80, LineTo 10 80, ClosePath ]
  "#ef4444"

-- Compose a scene incrementally
scene :: PaintScene
scene =
  addInstruction
    (addInstruction (emptyScene 100 100 "#fff") (makeRect 0 0 100 100 "#eee"))
    triangle
```

## How it fits in the stack

| Layer | Package |
|-------|---------|
| Field arithmetic | `gf256` |
| Polynomials | `polynomial` |
| Error correction | `reed-solomon` |
| **Paint IR** | **`paint-instructions`** <- you are here |
| Barcode layout | `barcode-2d` |

## Design notes

- All types derive `Show` and `Eq` — easy to inspect and test.
- Metadata fields (`prMeta`, `ppMeta`, `psMeta`) use `Map String Value`
  (aeson `Value`) so producers can attach arbitrary annotations. The
  renderer ignores them.
- `addInstruction` is pure (returns a new scene). Use `foldl addInstruction`
  to build a scene from a list of instructions.

## Running the tests

```bash
cd code/packages/haskell
mise exec -- cabal test paint-instructions
```
