# EAN-13

## Overview

This spec defines a dependency-free **EAN-13** encoder package for the
coding-adventures monorepo.

EAN-13 is closely related to UPC-A, but adds a leading digit that controls the
parity pattern used for the left half of the barcode. That makes it an
excellent next step once UPC-A is implemented and understood.

The package should emit backend-neutral draw instructions through the shared
1D barcode abstraction.

## Scope

### V1 In Scope

- 13-digit numeric payloads
- required check digit calculation
- parity pattern selection from the leading digit
- guard patterns
- draw-instructions output through the shared 1D layer
- visualizer-friendly intermediate structures

### V1 Out of Scope

- EAN-2 and EAN-5 supplements
- GTIN registry lookup
- scanner-side decoding

## Input Rules

The package should accept either:

- a 12-digit payload and compute the check digit
- a full 13-digit code and validate the supplied check digit

Any non-digit input must be rejected.

## Check Digit

EAN-13 uses the same modulo-10 weighted checksum idea as UPC-A, but applied to
the first 12 digits:

1. Starting from the right, multiply alternating digits by `3` and `1`.
2. Sum the results.
3. The check digit is the amount needed to reach the next multiple of 10.

## Barcode Structure

EAN-13 always encodes 95 modules:

- start guard: `101`
- left six visible digits: 6 x 7 modules
- center guard: `01010`
- right six digits: 6 x 7 modules
- end guard: `101`

The first digit is not encoded directly as bars. Instead, it selects which
left-side parity pattern is used.

### Left Parity Table

| First Digit | Left Pattern |
| --- | --- |
| 0 | `LLLLLL` |
| 1 | `LLGLGG` |
| 2 | `LLGGLG` |
| 3 | `LLGGGL` |
| 4 | `LGLLGG` |
| 5 | `LGGLLG` |
| 6 | `LGGGLL` |
| 7 | `LGLGLG` |
| 8 | `LGLGGL` |
| 9 | `LGGLGL` |

### Digit Encodings

| Digit | L-code   | G-code   | R-code   |
| --- | --- | --- | --- |
| 0 | `0001101` | `0100111` | `1110010` |
| 1 | `0011001` | `0110011` | `1100110` |
| 2 | `0010011` | `0011011` | `1101100` |
| 3 | `0111101` | `0100001` | `1000010` |
| 4 | `0100011` | `0011101` | `1011100` |
| 5 | `0110001` | `0111001` | `1001110` |
| 6 | `0101111` | `0000101` | `1010000` |
| 7 | `0111011` | `0010001` | `1000100` |
| 8 | `0110111` | `0001001` | `1001000` |
| 9 | `0001011` | `0010111` | `1110100` |

## Intermediate Structures

The package should expose:

- normalized digits
- whether the check digit was computed or supplied
- the selected left parity pattern
- per-digit encoded bit patterns
- 1D run stream
- shared 1D layout metadata

## Public API

```typescript
function normalizeEan13(data: string): string;
function computeEan13CheckDigit(payload12: string): string;
function encodeEan13(data: string): EncodedDigit[];
function expandEan13Runs(data: string): Barcode1DRun[];
function drawEan13(data: string, options?: DrawOptions): DrawScene;
```

## Teaching Value

EAN-13 is useful because it shows:

- how a barcode can encode data indirectly through parity
- how the same digit can have multiple visual encodings
- how scanners use left/right asymmetry and guards for orientation
