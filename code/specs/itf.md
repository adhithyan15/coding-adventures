# ITF

## Overview

This spec defines a dependency-free **Interleaved 2 of 5 (ITF)** encoder
package for the coding-adventures monorepo.

ITF is the first format in this track where pairs of digits are interleaved:
one digit controls the bars and the next digit controls the spaces.

The package should emit backend-neutral draw instructions through the shared
1D barcode abstraction.

## Scope

### V1 In Scope

- numeric-only input
- even-length payload requirement
- standard start and stop patterns
- interleaved pair encoding
- draw-instructions output through the shared 1D layer

### V1 Out of Scope

- ITF-14 bearer bars
- GTIN packaging semantics
- optional check digits beyond what a caller provides

## Input Rules

The package should accept a string of digits with even length.

Odd-length input must be rejected in V1 rather than padded implicitly.

## Symbol Structure

ITF uses:

- a start pattern of `1010`
- repeated digit pairs
- a stop pattern of `11101`

Each digit is represented by five elements with exactly two wide elements and
three narrow elements.

For each digit pair:

- the first digit contributes the widths of the bars
- the second digit contributes the widths of the spaces

Those are interleaved into ten runs.

## Digit Patterns

The package should store the standard five-element digit table in source:

| Digit | Pattern |
| --- | --- |
| 0 | `NNWWN` |
| 1 | `WNNNW` |
| 2 | `NWNNW` |
| 3 | `WWNNN` |
| 4 | `NNWNW` |
| 5 | `WNWNN` |
| 6 | `NWWNN` |
| 7 | `NNNWW` |
| 8 | `WNNWN` |
| 9 | `NWNWN` |

The implementation should convert `N` and `W` into numeric module widths for
the shared 1D layer.

## Intermediate Structures

The package should expose:

- normalized digits
- digit pairs
- per-pair interleaved patterns
- expanded 1D run stream
- shared 1D layout metadata

## Public API

```typescript
function normalizeItf(data: string): string;
function encodeItf(data: string): EncodedPair[];
function expandItfRuns(data: string): Barcode1DRun[];
function drawItf(data: string, options?: DrawOptions): DrawScene;
```

## Teaching Value

ITF is useful because it shows:

- how two digits can share one visual block
- how bars and spaces can carry different digits at the same time
- why even-length constraints exist in some symbologies
