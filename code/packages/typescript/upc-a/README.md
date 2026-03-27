# @coding-adventures/upc-a

Dependency-free UPC-A encoder that stops at backend-neutral draw instructions.

## What UPC-A Teaches

UPC-A is the first retail barcode in the repository's barcode track. It is a
good teaching format because it makes several ideas visible at once:

- numeric-only payloads
- a required check digit
- start, center, and end guard patterns
- left-side and right-side digit encodings

## Pipeline

```text
11 or 12 digits
  -> normalize and validate
  -> compute or validate the check digit
  -> encode 12 digits using UPC-A left/right tables
  -> expand into shared 1D barcode runs
  -> translate into draw instructions
  -> hand the scene to any renderer backend
```

## Usage

```typescript
import { drawUpcA } from "@coding-adventures/upc-a";
import { renderSvg } from "@coding-adventures/draw-instructions-svg";

const scene = drawUpcA("03600029145");
const svg = renderSvg(scene);
```

The package also exposes intermediate helpers such as:

- `computeUpcACheckDigit()`
- `normalizeUpcA()`
- `encodeUpcA()`
- `expandUpcARuns()`

## Why It Depends On `barcode-1d`

UPC-A should not know how to serialize SVG or place rectangles directly.
Instead it focuses on the retail barcode rules, then hands its run stream to
the shared 1D barcode layer.

## Development

```bash
bash BUILD
```
