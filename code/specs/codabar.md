# Codabar

## Overview

This spec defines a dependency-free **Codabar** encoder package for the
coding-adventures monorepo.

Codabar is a useful companion to Code 39 because it is still easy to explain,
but it introduces explicit start/stop symbols chosen from a small set rather
than a single universal delimiter.

The package should emit backend-neutral draw instructions through the shared
1D barcode abstraction.

## Scope

### V1 In Scope

- digits `0-9`
- punctuation `- $ : / . +`
- required start/stop symbols chosen from `A B C D`
- run expansion into the shared 1D model
- draw-instructions output through the shared 1D layer

### V1 Out of Scope

- optional checksum conventions
- alternate start/stop aliases such as `E N * T`
- format-specific application profiles

## Input Rules

The package should accept either:

- a full Codabar string with explicit start and stop symbols
- a body string plus explicit `start` and `stop` options

If the input body contains `A B C D`, they must be rejected unless they occupy
the first and last positions as delimiters.

## Symbol Structure

Each Codabar symbol contains:

- 7 elements
- 4 bars
- 3 spaces

Adjacent symbols are separated by an extra narrow space.

The implementation should preserve the symbol-level pattern table in source
form and expand it into numeric module widths for the shared 1D layer.

## Symbol Table

The package should implement the standard symbol mappings:

- digits `0-9`
- `- $ : / . +`
- start/stop `A B C D`

The exact narrow/wide table belongs in source code and tests.

## Intermediate Structures

The package should expose:

- normalized full symbol string
- start symbol and stop symbol
- per-symbol narrow/wide pattern
- expanded 1D run stream
- shared 1D layout metadata

## Public API

```typescript
function normalizeCodabar(
  data: string,
  options?: { start?: "A" | "B" | "C" | "D"; stop?: "A" | "B" | "C" | "D" },
): string;
function encodeCodabar(data: string, options?: NormalizeOptions): EncodedSymbol[];
function expandCodabarRuns(data: string, options?: NormalizeOptions): Barcode1DRun[];
function drawCodabar(data: string, options?: DrawOptions & NormalizeOptions): DrawScene;
```

## Teaching Value

Codabar is useful because it shows:

- how barcode standards can reserve multiple start/stop symbols
- how inter-character gaps affect the final run stream
- how a self-checking format can exist without a universal checksum
