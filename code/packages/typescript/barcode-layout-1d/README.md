# @coding-adventures/barcode-layout-1d

Shared geometry and paint-layout helpers for linear barcode symbologies.

This package exists so each 1D symbology does not have to reinvent the same
left-to-right layout work. A barcode package should decide:

- what its valid input looks like
- how to encode symbols
- how to compute checksums
- which parts are guards, starts, stops, or data

Once that work is done, most linear barcodes become the same shape:

```text
bar / space / bar / space / ...
```

with widths measured in modules.

## What This Package Owns

- the shared `Barcode1DRun` model
- binary-pattern and width-pattern helpers
- quiet-zone aware layout calculation
- translation from runs into backend-neutral `PaintScene` output

## What It Does Not Own

- UPC-A checksum rules
- EAN-13 parity tables
- Codabar guard semantics
- ITF digit interleaving
- Code 128 code-set logic

Those belong in the symbology packages.

## Usage

```typescript
import {
  runsFromBinaryPattern,
  layoutBarcode1D,
} from "@coding-adventures/barcode-layout-1d";

const runs = runsFromBinaryPattern("101", {
  sourceLabel: "start",
  sourceIndex: -1,
  role: "guard",
});

const scene = layoutBarcode1D(runs, {
  label: "Demo barcode",
});
```

## Why It Matters

This package is the reason we can keep the pipeline clean:

```text
symbology rules
  -> linear barcode runs
  -> paint scene
  -> paint VM
  -> PNG / other backend
```

That same split is what makes the repository's barcode visualizers explainable
instead of black-box image generators.

## Development

```bash
bash BUILD
```

## Portable v1 API

`expandBinaryV1`, `expandWidthV1`, `computeLayoutV1`, and `projectSceneV1`
implement the bounded language-neutral contract in
`code/specs/barcode-layout-1d-v1.md`. The adapter uses stable error IDs,
Unicode-scalar validation, checked integer geometry, canonical string
metadata, and fresh result objects. `projectSceneV1` is rectangle-only and
rejects either human-readable text request form before layout or backend work.

The older helpers remain available for existing callers. New portable code
should use the v1 entry points.
