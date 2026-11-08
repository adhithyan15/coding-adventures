# @coding-adventures/pbt

Standalone, zero-runtime-dependency TypeScript Level 2 property-based testing engine with generator algebra, integrated shrinking, and deterministic replay (VER05).

## Features

- **Zero Runtime Dependencies**: Strict `"dependencies": {}` guarantee.
- **Generator Algebra**: Monadic combinators (`map`, `filter`, `flatMap`, `list`, `oneOf`, `tuple`, `constant`, `integer`, `bytes`, `bool`, `u8`, `u64`).
- **Integrated Shrinking**:
  - `shrinkInt`: Euclidean division binary search with linear fallback.
  - `shrinkList`: 3-phase reduction (bisection, deletion, element-wise integer shrinking).
  - `shrinkBytes`: Multi-phase byte sequence reduction.
- **Deterministic Replay**: SplitMix64 PRNG with isolated sub-seed derivation ensuring reproducible counterexamples.
- **Closed Failure Vocabulary**: Strict classification (`property-counterexample`, `property-exhausted`, `property-timeout`, `property-panic`).
- **Privacy-Preserving Reporting**: Hash-based digest redaction for CI environments to prevent leaking sensitive fixtures or inputs.

## Installation

```bash
npm install @coding-adventures/pbt
```

## Quick Start

```typescript
import { Gen, assertProperty } from "@coding-adventures/pbt";

// Define a property using generator algebra
assertProperty(
  Gen.list(0, 50, Gen.integer(-1000, 1000)),
  (nums) => {
    const rev = [...nums].reverse().reverse();
    return rev.every((val, idx) => val === nums[idx]);
  },
  { numTests: 200, seed: 12345n }
);
```

## Specification Conformance

Conforms to VER05 specification and passes all 25 canonical test vectors in `code/fixtures/pbt-v1/cases.json`.
