# Stats (Swift)

Descriptive statistics, frequency analysis, and cryptanalysis helpers for the coding-adventures project.

## Overview

This package provides pure functions for:

1. **Descriptive statistics** -- mean, median, mode, variance, standardDeviation, statsMin, statsMax, statsRange
2. **Frequency analysis** -- frequencyCount, frequencyDistribution, chiSquared, chiSquaredText
3. **Cryptanalysis helpers** -- indexOfCoincidence, entropy, englishFrequencies

## Usage

```swift
import Stats

// Descriptive statistics
let m = try mean([1, 2, 3, 4, 5])               // 3.0
let med = try median([2, 4, 4, 4, 5, 5, 7, 9])  // 4.5
let v = try variance([2, 4, 4, 4, 5, 5, 7, 9])  // 4.571... (sample)
let vp = try variance([2, 4, 4, 4, 5, 5, 7, 9], population: true)  // 4.0

// Frequency analysis
let counts = frequencyCount("Hello World")
print(counts["L"]!)  // 3

let chi2 = try chiSquared(observed: [10, 20, 30], expected: [20, 20, 20])
print(chi2)  // 10.0

// Cryptanalysis
let ic = indexOfCoincidence("AABB")
print(ic)  // 0.333...

let h = entropy("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
print(h)  // ~4.700
```

## Design Principles

- **Pure functions.** No side effects, no mutation of inputs.
- **No external dependencies.** Pure math only (Foundation for sqrt/log).
- **Population vs sample.** Variance and standard deviation default to sample (Bessel-corrected). Pass `population: true` for population statistics.
- **Swift naming conventions.** Functions use camelCase (e.g., `frequencyCount` instead of `frequency_count`).

## Running Tests

```sh
swift test --enable-code-coverage --verbose
```
