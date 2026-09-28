# @coding-adventures/itf

Interleaved 2 of 5 encoder that emits backend-neutral paint instructions.

## What ITF Teaches

ITF is the first barcode in this track where bars and spaces carry different
digits at the same time.

For each digit pair:

- the first digit chooses the bar widths
- the second digit chooses the space widths
- the two patterns are interleaved into one visual block

That is why ITF requires an even number of digits.

## Usage

```typescript
import { drawItf } from "@coding-adventures/itf";
const scene = drawItf("123456");
```

Intermediate helpers include:

- `normalizeItf()`
- `encodeItf()`
- `expandItfRuns()`

## Architecture

This package owns the ITF digit-pair rules and start/stop patterns. The shared
1D geometry and SVG serialization live in sibling packages so the symbology
stays easy to read.

Validation follows `barcode-symbologies-v1`; stable IDs are exposed through
`InvalidItfInputError.errorId`.

## Development

```bash
bash BUILD
```
