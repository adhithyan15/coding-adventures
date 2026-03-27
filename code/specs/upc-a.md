# UPC-A

## Overview

This spec defines a dependency-free **UPC-A** encoder package for the
coding-adventures monorepo.

UPC-A will be the first retail barcode format in the barcode track. Unlike
Code 39, UPC-A is numeric-only and includes a required check digit, guard
patterns, and left/right digit encoding rules that make it a strong teaching
step after the simpler Code 39 implementation.

The package should emit backend-neutral draw instructions through the shared
1D barcode abstraction.

## Scope

### V1 In Scope

- 12-digit numeric payloads
- required check digit calculation
- start, middle, and end guard patterns
- left/right digit encoding
- draw-instructions output through the shared 1D layer
- visualizer-friendly intermediate structures

### V1 Out of Scope

- UPC-E compression
- add-on supplements
- GS1 application identifiers
- scanner-side decoding

## Input Rules

The package should accept either:

- an 11-digit payload and compute the check digit
- a full 12-digit code and validate the supplied check digit

Any non-digit input must be rejected.

## Check Digit

UPC-A uses a modulo-10 check digit.

For the first 11 digits:

1. Sum digits in odd positions and multiply that sum by 3.
2. Sum digits in even positions.
3. Add the two results.
4. The check digit is the amount needed to reach the next multiple of 10.

## Barcode Structure

UPC-A always encodes 95 modules:

- start guard: `101`
- left six digits: 6 x 7 modules
- center guard: `01010`
- right six digits: 6 x 7 modules
- end guard: `101`

The left half uses L patterns. The right half uses R patterns.

### Digit Encodings

Each digit maps to a 7-module pattern.

| Digit | L-code   | R-code   |
| --- | --- | --- |
| 0 | `0001101` | `1110010` |
| 1 | `0011001` | `1100110` |
| 2 | `0010011` | `1101100` |
| 3 | `0111101` | `1000010` |
| 4 | `0100011` | `1011100` |
| 5 | `0110001` | `1001110` |
| 6 | `0101111` | `1010000` |
| 7 | `0111011` | `1000100` |
| 8 | `0110111` | `1001000` |
| 9 | `0001011` | `1110100` |

## Intermediate Structures

The package should expose:

- normalized digits
- whether the check digit was computed or supplied
- left/right encoded digit patterns
- 1D run stream
- shared 1D layout metadata

## Public API

Language-neutral pseudocode:

```typescript
function normalizeUpcA(data: string): string;
function computeUpcACheckDigit(payload11: string): string;
function encodeUpcA(data: string): EncodedDigit[];
function expandUpcARuns(data: string): Barcode1DRun[];
function drawUpcA(data: string, options?: DrawOptions): DrawScene;
```

## Teaching Value

UPC-A is the first barcode in this track that clearly shows:

- how guard patterns help a scanner orient itself
- how a check digit catches data corruption
- how equal-width modules differ from wide/narrow symbologies
