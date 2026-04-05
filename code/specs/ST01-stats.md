# ST01 — Statistics

## Overview

The `stats` package provides descriptive statistics, frequency analysis,
and cryptanalysis helper functions. It serves two purposes:

1. **General-purpose statistics.** Mean, median, mode, variance, standard
   deviation — usable by any package in the repo.
2. **Cryptanalysis toolkit.** Chi-squared tests, index of coincidence,
   Shannon entropy, and English frequency tables — extracted from the
   inline implementations in CR00 (Caesar cipher) and designed for reuse
   by CR03 (Vigenere), future cipher packages, and ML workloads.

Every function has a scalar variant (operating on arrays/lists) and where
applicable a matrix variant (operating on Matrix objects from ML03).

## Design Principles

1. **Tree-shakeable.** Each function is independently importable. In
   TypeScript, each function lives in its own file with a barrel export.
   In Rust, each function is a separate public module. Other languages
   follow their idiomatic equivalent.
2. **No external dependencies.** Pure math only. Matrix variants depend
   on ML03 matrix (with extensions).
3. **Pure functions.** No side effects, no mutation of inputs.
4. **Population vs sample.** Variance and standard deviation accept a
   boolean flag (default: sample/Bessel-corrected). Population=true
   divides by n. Sample (default) divides by n-1.

## Interface Contract

### Descriptive Statistics (Scalar)

All scalar functions take an array/list of floats.

| Function | Signature | Description |
|----------|-----------|-------------|
| `mean` | `(values) -> float` | Sum / n. |
| `median` | `(values) -> float` | Middle value. Average of two middle if even. |
| `mode` | `(values) -> float` | Most frequent. First occurrence wins ties. |
| `variance` | `(values, population=false) -> float` | Sum of squared deviations / d (d=n if population, n-1 if sample). |
| `standard_deviation` | `(values, population=false) -> float` | Square root of variance. |
| `min` | `(values) -> float` | Minimum value. |
| `max` | `(values) -> float` | Maximum value. |
| `range` | `(values) -> float` | max - min. |

### Descriptive Statistics (Matrix)

Matrix functions take a Matrix and optional axis. axis=None operates on
all elements. axis=0 reduces along rows (per-column). axis=1 reduces
along columns (per-row).

| Function | Signature | Description |
|----------|-----------|-------------|
| `mean_matrix` | `(matrix, axis=None) -> float or Matrix` | Mean over axis. |
| `variance_matrix` | `(matrix, axis=None) -> float or Matrix` | Variance over axis. |
| `std_matrix` | `(matrix, axis=None) -> float or Matrix` | Standard deviation over axis. |

### Frequency Analysis

| Function | Signature | Description |
|----------|-----------|-------------|
| `frequency_count` | `(text) -> map[char, int]` | Count each letter (case-insensitive, A-Z only). |
| `frequency_distribution` | `(text) -> map[char, float]` | Proportion of each letter (counts / total). |
| `chi_squared` | `(observed, expected) -> float` | Sum of (O-E)^2/E for parallel arrays. |
| `chi_squared_text` | `(text, expected_freq) -> float` | Chi-squared of text vs expected frequency table. |

### Cryptanalysis Helpers

| Function | Signature | Description |
|----------|-----------|-------------|
| `index_of_coincidence` | `(text) -> float` | IC = Sum(n_i*(n_i-1)) / (N*(N-1)). English ~0.0667, random ~0.0385. |
| `entropy` | `(text) -> float` | Shannon: -Sum(p_i * log2(p_i)). |

### Constants

| Constant | Type | Description |
|----------|------|-------------|
| `ENGLISH_FREQUENCIES` | `map[char, float]` | Standard English letter frequencies (a=0.08167 ... z=0.00074). |

## Worked Examples

### Scalar Statistics

```text
values = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]

mean(values)                      -> 5.0
median(values)                    -> 4.5
mode(values)                      -> 4.0
variance(values)                  -> 4.571428...  (sample, n-1=7)
variance(values, population=true) -> 4.0          (population, n=8)
standard_deviation(values)        -> 2.138...
range(values)                     -> 7.0
```

### Index of Coincidence

```text
english_text = "THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG"
# IC ~ 0.0655 (close to English expected 0.0667)

random_text = "XKQZJWMFLPRABINOCEDGHSTYUV"
# IC ~ 0.0385 (close to random expected 1/26 = 0.0385)
```

### Chi-Squared

```text
observed = [10, 20, 30]
expected = [20, 20, 20]
chi_squared(observed, expected)
  = (10-20)^2/20 + (20-20)^2/20 + (30-20)^2/20
  = 5.0 + 0.0 + 5.0
  = 10.0
```

## Parity Test Vectors

- **mean:** `[1,2,3,4,5]` -> `3.0`
- **variance (sample):** `[2,4,4,4,5,5,7,9]` -> `4.571428571428571`
- **variance (population):** `[2,4,4,4,5,5,7,9]` -> `4.0`
- **chi_squared:** `[10,20,30]` vs `[20,20,20]` -> `10.0`
- **index_of_coincidence:** `"AABB"` -> counts A=2 B=2, N=4, IC = 4/12 = `0.333...`
- **entropy uniform:** 26 equal letters -> `log2(26) ~ 4.700`

## Tree-Shakeable File Layout (TypeScript)

```
stats/src/
  mean.ts          median.ts       mode.ts
  variance.ts      standard_deviation.ts
  min.ts           max.ts          range.ts
  mean_matrix.ts   variance_matrix.ts   std_matrix.ts
  frequency_count.ts   frequency_distribution.ts
  chi_squared.ts       chi_squared_text.ts
  index_of_coincidence.ts   entropy.ts
  english_frequencies.ts
  index.ts  (barrel re-export)
```

## Package Matrix

| Language | Package Directory | Module/Namespace |
|----------|-------------------|------------------|
| Python | `code/packages/python/stats/` | `stats` |
| Go | `code/packages/go/stats/` | `stats` |
| Ruby | `code/packages/ruby/stats/` | `CodingAdventures::Stats` |
| TypeScript | `code/packages/typescript/stats/` | `@coding-adventures/stats` |
| Rust | `code/packages/rust/stats/` | `stats` |
| Elixir | `code/packages/elixir/stats/` | `CodingAdventures.Stats` |
| Lua | `code/packages/lua/stats/` | `coding_adventures.stats` |
| Perl | `code/packages/perl/stats/` | `CodingAdventures::Stats` |
| Swift | `code/packages/swift/stats/` | `Stats` |

**Dependencies:** ML03 Matrix (for matrix variants only). Scalar functions
and frequency/cryptanalysis functions have zero dependencies.
