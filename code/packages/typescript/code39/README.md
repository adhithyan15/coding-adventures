# @ca/code39

Dependency-free Code 39 encoder that emits backend-neutral draw instructions.

This package does not know about SVG directly. It produces generic scenes that
other renderer packages can consume.

## What Code 39 Is

Code 39 is a classic 1D barcode symbology.

It encodes:

- digits `0-9`
- uppercase letters `A-Z`
- a small punctuation set: `- . space $ / + %`

and uses `*` as the start/stop symbol.

Each encoded character becomes a sequence of:

- 5 bars
- 4 spaces

with a mix of narrow and wide elements.

## Pipeline

This package keeps the full process explicit:

```text
input string
  -> normalize to Code 39 rules
  -> encode symbols
  -> expand to runs
  -> translate runs to DrawScene
```

That split matters because each step teaches something different:

- normalization explains the allowed alphabet
- encoded characters explain symbol mapping
- runs explain what scanners conceptually move across
- draw scenes explain geometry without tying us to SVG

## Usage

```typescript
import { drawCode39 } from "@ca/code39";
import { renderSvg } from "@ca/draw-instructions-svg";

const scene = drawCode39("HELLO-123");
const svg = renderSvg(scene);
```

You can also inspect the intermediate barcode-domain structures:

```typescript
import {
  normalizeCode39,
  encodeCode39,
  expandCode39Runs,
  drawCode39,
} from "@ca/code39";

const normalized = normalizeCode39("hello-123");
const encoded = encodeCode39(normalized);
const runs = expandCode39Runs(normalized);
const scene = drawCode39(normalized);
```

## Why This Package Stops At Draw Instructions

That boundary is intentional.

If Code 39 returned SVG directly, then:

- every new backend would require changing the symbology package
- the encoding logic would get mixed with output-format concerns
- 2D formats would be harder to add cleanly

Instead, this package only decides what the barcode geometry should be.

## Render Config

The render config controls geometry, not encoding:

- `narrowUnit`
- `wideUnit`
- `barHeight`
- `quietZoneUnits`
- `includeHumanReadableText`

This means the same encoded barcode can be drawn at different sizes without
changing the underlying barcode logic.

## Learning Value

This package is meant to be read, not just imported.

Someone reading the source should be able to answer:

- why Code 39 uses start/stop markers
- how a character becomes a width pattern
- how a width pattern becomes alternating bars and spaces
- why bars become rectangles in a generic drawing scene
