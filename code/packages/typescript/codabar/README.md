# @coding-adventures/codabar

Dependency-free Codabar encoder that emits backend-neutral draw instructions.

## What Codabar Teaches

Codabar is simpler than the dense retail formats, but it still introduces a
useful concept that Code 39 does not emphasize: the caller can choose distinct
start and stop symbols.

This package supports:

- digits `0-9`
- punctuation `- $ : / . +`
- start/stop guards `A B C D`

## Usage

```typescript
import { drawCodabar } from "@coding-adventures/codabar";
import { renderSvg } from "@coding-adventures/draw-instructions-svg";

const scene = drawCodabar("40156");
const svg = renderSvg(scene);
```

If the caller passes only a body string, the package wraps it with `A ... A` by
default. It also accepts full Codabar strings such as `B40156D`.

Intermediate helpers include:

- `normalizeCodabar()`
- `encodeCodabar()`
- `expandCodabarRuns()`

## Architecture

Codabar-specific symbol rules stay here. Rectangle placement and scene
construction live in `@coding-adventures/barcode-1d`, and SVG serialization
lives in `@coding-adventures/draw-instructions-svg`.

## Development

```bash
bash BUILD
```
