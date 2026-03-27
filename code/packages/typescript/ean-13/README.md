# @coding-adventures/ean-13

Dependency-free EAN-13 encoder that emits backend-neutral draw instructions.

## What Makes EAN-13 Interesting

EAN-13 is closely related to UPC-A, but the first digit is encoded indirectly.
It selects the parity pattern used by the next six digits on the left half of
the barcode.

That makes EAN-13 a strong teaching format for:

- parity-controlled encoding
- retail guard patterns
- modulo-10 check digits
- orientation-friendly left/right asymmetry

## Pipeline

```text
12 or 13 digits
  -> normalize and validate
  -> compute or validate the check digit
  -> choose the left parity pattern from the leading digit
  -> expand to shared 1D runs
  -> translate to draw instructions
  -> hand the scene to any renderer backend
```

## Usage

```typescript
import { drawEan13 } from "@coding-adventures/ean-13";
import { renderSvg } from "@coding-adventures/draw-instructions-svg";

const scene = drawEan13("400638133393");
const svg = renderSvg(scene);
```

Useful intermediate helpers:

- `computeEan13CheckDigit()`
- `normalizeEan13()`
- `leftParityPattern()`
- `encodeEan13()`
- `expandEan13Runs()`

## Development

```bash
bash BUILD
```
