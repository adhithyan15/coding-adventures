# barcode-2d (Haskell)

Shared 2D barcode abstraction: `ModuleGrid` + layout engine.

## What is this?

Every 2D barcode format shares the same structure: a rectangular grid of
dark and light modules. This package provides:

1. **`ModuleGrid`** — the universal intermediate representation (a 2D boolean
   grid).
2. **`layout`** — converts a `ModuleGrid` to a `PaintScene` ready for the
   PaintVM renderer.

## Pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → layout              ← THIS PACKAGE
  → PaintScene          ← consumed by paint-vm
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout` are in "module units". Only `layout` converts
to pixels by multiplying by `moduleSizePx`. Encoders never need to know about
screen resolution.

## Module shapes

| Shape | Barcode formats | Rendering |
|---|---|---|
| `Square` | QR Code, Data Matrix, Aztec, PDF417 | `PaintRect` per dark module |
| `Hex` | MaxiCode (ISO/IEC 16023) | `PaintPath` per dark module |

## Usage

```haskell
import Barcode2D
import PaintInstructions (PaintScene)

-- 1. Create an all-light 21x21 grid (QR Code v1 size)
let grid0 = makeModuleGrid 21 21 Square

-- 2. Set some modules dark (encoder would do this systematically)
let grid1 = either error id (setModule grid0 0 0 True)
let grid2 = either error id (setModule grid1 0 1 True)

-- 3. Convert to a PaintScene with default config (10px modules, 4-module quiet zone)
let scene = either error id (layout grid2 defaultBarcode2DLayoutConfig)
-- scene is now ready for any paint-vm backend
```

## Configuration

```haskell
data Barcode2DLayoutConfig = Barcode2DLayoutConfig
  { moduleSizePx      :: Int     -- default 10  (pixels per module)
  , quietZoneModules  :: Int     -- default 4   (quiet zone in module units)
  , foreground        :: String  -- default "#000000" (dark module color)
  , background        :: String  -- default "#ffffff" (light / canvas color)
  , configModuleShape :: ModuleShape  -- default Square
  }
```

Override just the fields you need:

```haskell
let cfg = defaultBarcode2DLayoutConfig { moduleSizePx = 5, quietZoneModules = 2 }
```

## Immutability

`setModule` is pure: it returns a new `ModuleGrid` without modifying the
original. Only the affected row is re-allocated; all other row `Vector`s are
shared. This makes encoder backtracking (e.g. trying all 8 QR mask patterns)
free from extra allocation.

## Annotations

`AnnotatedModuleGrid` wraps a `ModuleGrid` with per-module role information
(finder, timing, data, ECC, etc.) for visualizers. The renderer ignores
annotations — it only reads `gridModules`.

## Dependency

Depends on `paint-instructions` (local sibling package) for `PaintScene`,
`PaintRect`, `PaintPath`, and `PathCommand`.

## License

MIT
