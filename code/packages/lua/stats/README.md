# Stats (Lua)

Descriptive statistics, frequency analysis, and cryptanalysis helpers for the coding-adventures project.

## Overview

This package provides pure functions for:

1. **Descriptive statistics** -- mean, median, mode, variance, standard deviation, min, max, range
2. **Frequency analysis** -- frequency_count, frequency_distribution, chi_squared, chi_squared_text
3. **Cryptanalysis helpers** -- index_of_coincidence, entropy, ENGLISH_FREQUENCIES

## Usage

```lua
local Stats = require("coding_adventures.stats")

-- Descriptive statistics
print(Stats.mean({1, 2, 3, 4, 5}))           -- 3.0
print(Stats.median({2, 4, 4, 4, 5, 5, 7, 9})) -- 4.5
print(Stats.variance({2, 4, 4, 4, 5, 5, 7, 9})) -- 4.571... (sample)
print(Stats.variance({2, 4, 4, 4, 5, 5, 7, 9}, true)) -- 4.0 (population)

-- Frequency analysis
local counts = Stats.frequency_count("Hello World")
print(counts["L"])  -- 3

local chi2 = Stats.chi_squared({10, 20, 30}, {20, 20, 20})
print(chi2)  -- 10.0

-- Cryptanalysis
local ic = Stats.index_of_coincidence("AABB")
print(ic)  -- 0.333...

local h = Stats.entropy("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
print(h)  -- ~4.700
```

## Design Principles

- **Pure functions.** No side effects, no mutation of inputs.
- **No external dependencies.** Pure math only.
- **Population vs sample.** Variance and standard deviation default to sample (Bessel-corrected). Pass `true` as the second argument for population statistics.

## Running Tests

```sh
cd tests && busted . --verbose --pattern=test_
```
