# qr-code (TypeScript)

QR Code encoder — ISO/IEC 18004:2015 compliant.

Encodes any UTF-8 string into a scannable QR Code. Outputs an abstract
`ModuleGrid` (no pixel coordinates yet) that can be passed to `barcode-2d`'s
`layout()` function for pixel-level rendering, or to `renderSvg()` for a
one-shot SVG string.

## Pipeline

```
input string
  → mode selection    (numeric / alphanumeric / byte)
  → version selection (smallest version 1–40 that fits at the ECC level)
  → bit stream        (mode indicator + char count + data bits + padding)
  → blocks + RS ECC   (GF(256) b=0 convention, polynomial 0x11D)
  → interleave        (data CWs, then ECC CWs, round-robin across blocks)
  → grid init         (finder × 3, separator, timing, alignment, format, dark)
  → zigzag placement  (two-column snake from bottom-right corner)
  → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
  → finalize          (format info + version info v7+)
  → ModuleGrid        (abstract boolean grid, true = dark)
```

## Usage

```typescript
import { encode, encodeAndLayout, renderSvg, type EccLevel } from "@coding-adventures/qr-code";

// Just the module grid (abstract units — no pixels):
const grid = encode("https://example.com", "M");
// grid.rows === grid.cols === 25 (version 2)

// Grid → pixel-resolved PaintScene (via barcode-2d):
const scene = encodeAndLayout("https://example.com", "M", { moduleSizePx: 10 });

// Direct SVG string (one call):
const svg = renderSvg("https://example.com", "M");
document.body.innerHTML = svg;
```

## API

### `encode(input, eccLevel) → ModuleGrid`

Encodes a UTF-8 string into a QR Code module grid.

- Returns a `(4V+17) × (4V+17)` grid where `true` = dark module.
- Throws `InputTooLongError` if the input exceeds version-40 capacity.
- Selects the minimum version that fits the input at the ECC level.
- Mode is auto-selected: numeric → alphanumeric → byte.

### `encodeAndLayout(input, eccLevel, config?) → PaintScene`

Encodes and converts to a pixel-resolved `PaintScene` via `barcode-2d`'s
`layout()`. The `config` parameter accepts any `Barcode2DLayoutConfig` fields.

### `renderSvg(input, eccLevel, config?) → string`

Convenience: encode → layout → SVG string. Returns a complete `<svg>…</svg>`.

### `explain(input, eccLevel) → AnnotatedModuleGrid`

Encodes with per-module role annotations (for visualizers). v0.1.0 returns
null annotations; full annotation is v0.2.0.

## ECC Levels

| Level | Recovery | Notes                                  |
|-------|----------|----------------------------------------|
| L     | ~7%      | Highest data density                   |
| M     | ~15%     | General-purpose default                |
| Q     | ~25%     | Moderate damage expected               |
| H     | ~30%     | High damage risk, or overlaid logo     |

## Error Types

- `InputTooLongError` — input exceeds version-40 capacity at the chosen ECC level.

## Dependencies

- `barcode-2d`: `ModuleGrid` type, `layout()` pixel geometry
- `gf256`: GF(256) field arithmetic (multiply, ALOG)
- `paint-vm-svg`: `renderToSvgString()` for SVG output

## Spec

See `code/specs/qr-code.md` for the full specification.
