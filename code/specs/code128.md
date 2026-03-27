# Code 128

## Overview

This spec defines a dependency-free **Code 128** encoder package for the
coding-adventures monorepo.

Code 128 is denser and more flexible than the earlier 1D formats in this
track. It uses variable code sets, a required checksum, and six run widths per
symbol instead of simple narrow/wide patterns.

The package should emit backend-neutral draw instructions through the shared
1D barcode abstraction.

## V1 Scope Decision

V1 should implement **Code Set B** end to end.

That means:

- printable ASCII input from space through tilde
- Start B
- required modulo-103 checksum
- Stop pattern
- shared draw-instructions pipeline

V1 should expose types and internal structure that make Code Sets A and C
natural future additions, but it does not need automatic code-set switching yet.

## Scope

### V1 In Scope

- Code Set B encoding
- Start B, checksum, and stop
- shared 1D run output
- draw-instructions output through the shared 1D layer

### V1 Out of Scope

- automatic switching between A, B, and C
- FNC application behavior
- GS1-128 conventions
- Code Set C compaction

## Input Rules

The package should accept printable ASCII characters:

- character codes `32` through `126`

Characters outside that range must be rejected in V1.

## Symbol Structure

Code 128 symbols are composed of six alternating run widths:

- bar
- space
- bar
- space
- bar
- space

Each data or control symbol totals 11 modules. The stop pattern totals 13
modules because it ends with an extra bar.

The barcode structure is:

- quiet zone
- start symbol
- zero or more data symbols
- checksum symbol
- stop symbol
- quiet zone

## Checksum

Code 128 uses modulo-103 weighting:

1. Start with the numeric value of the start symbol.
2. For each data symbol, multiply its value by its 1-based position.
3. Add the products to the start value.
4. Take the result modulo 103.
5. Append the symbol with that value as the checksum.

## Symbol Table

The implementation should store the full 107-pattern table in source code:

- values `0` through `102` for regular symbols and controls
- start codes `103`, `104`, `105`
- stop pattern

For V1, the public input mapping only needs Code Set B values.

## Intermediate Structures

The package should expose:

- normalized input
- selected start code
- per-character symbol values
- checksum value
- per-symbol six-width patterns
- expanded 1D run stream
- shared 1D layout metadata

## Public API

```typescript
function normalizeCode128B(data: string): string;
function encodeCode128B(data: string): EncodedSymbol[];
function computeCode128Checksum(encoded: EncodedSymbol[]): number;
function expandCode128Runs(data: string): Barcode1DRun[];
function drawCode128(data: string, options?: DrawOptions): DrawScene;
```

## Teaching Value

Code 128 is useful because it shows:

- dense module-based encoding
- weighted checksums stronger than simple modulo-10 retail checks
- the idea that a barcode standard can act like a small instruction set
