# @coding-adventures/stats

Statistics, frequency analysis, and cryptanalysis helpers for the
coding-adventures monorepo.

## What It Does

This package provides three categories of functions:

1. **Descriptive statistics** -- mean, median, mode, variance, standard
   deviation, min, max, range.
2. **Frequency analysis** -- letter frequency counting, frequency
   distributions, chi-squared tests.
3. **Cryptanalysis helpers** -- index of coincidence, Shannon entropy,
   and standard English letter frequency tables.

## How It Fits

This package is used by cipher packages (CR00 Caesar, CR03 Vigenere)
for frequency-based attacks, and by ML packages for basic statistical
operations. It has zero external dependencies.

## Usage

```typescript
import { mean, variance, chiSquared, indexOfCoincidence } from "@coding-adventures/stats";

// Descriptive statistics
mean([1, 2, 3, 4, 5]);           // => 3.0
variance([2, 4, 4, 4, 5, 5, 7, 9]); // => 4.571... (sample)
variance([2, 4, 4, 4, 5, 5, 7, 9], true); // => 4.0 (population)

// Chi-squared test
chiSquared([10, 20, 30], [20, 20, 20]); // => 10.0

// Cryptanalysis
indexOfCoincidence("AABB"); // => 0.333...
```

## Tree-Shaking

Each function lives in its own file. Import only what you need:

```typescript
import { mean } from "@coding-adventures/stats";
// Only mean.ts is bundled.
```

## Spec

See `code/specs/ST01-stats.md` for the full interface contract.
