# @coding-adventures/layout-text-measure-estimated

**Fast, zero-dependency, deterministic text measurer** using a fixed
character-width model.

## When to use

| Context | Use this measurer? |
|---|---|
| CI / unit tests | ✅ Yes — deterministic, no fonts needed |
| Server-side layout | ✅ Yes — no OS font access required |
| Headless environments | ✅ Yes — pure math, no APIs |
| First-pass progressive rendering | ✅ Yes — fast approximate layout |
| Browser Canvas pixel-accurate layout | ❌ No — use `layout-text-measure-canvas` |
| PDF with exact line breaks | ❌ No — use `layout-text-measure-rs` |

## Usage

```ts
import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";
import { font_spec } from "@coding-adventures/layout-ir";

const measurer = createEstimatedMeasurer();
const font = font_spec("Arial", 16);

// Single-line measurement
const r1 = measurer.measure("Hello world", font, null);
// r1.width ≈ 11 × 16 × 0.6 = 105.6
// r1.height = 16 × 1.2 = 19.2
// r1.lineCount = 1

// Multi-line measurement (constrained to 100px wide)
const r2 = measurer.measure("Hello world", font, 100);
// charsPerLine = floor(100 / 9.6) = 10
// lineCount = ceil(11 / 10) = 2
// r2.lineCount = 2
```

## Custom multiplier

The default multiplier of 0.6 is calibrated for proportional Latin fonts at
regular weight. Tune it for your content:

```ts
// Condensed font or narrow-character text
const narrow = createEstimatedMeasurer({ avgCharWidthMultiplier: 0.5 });

// Monospaced font (1 char = 1 em)
const mono = createEstimatedMeasurer({ avgCharWidthMultiplier: 1.0 });
```

## The model

```
estimated_width = text.length × font.size × avgCharWidthMultiplier

chars_per_line = floor(maxWidth / (font.size × avgCharWidthMultiplier))
line_count     = ceil(text.length / chars_per_line)
height         = line_count × font.size × font.lineHeight
```

## See also

- [UI09 — layout-text-measure spec](../../specs/UI09-layout-text-measure.md)
- `layout-text-measure-canvas` — accurate browser Canvas measurements
- `layout-text-measure-rs` — accurate Rust+fontdue measurements via FFI
