# stats (Rust)

Statistics, frequency analysis, and cryptanalysis helpers for the
coding-adventures monorepo.

## What It Does

This crate provides three categories of functions:

1. **Descriptive statistics** -- mean, median, mode, variance, standard
   deviation, min, max, range.
2. **Frequency analysis** -- letter frequency counting, frequency
   distributions, chi-squared tests.
3. **Cryptanalysis helpers** -- index of coincidence, Shannon entropy,
   and standard English letter frequency tables.

## How It Fits

Used by cipher crates (caesar-cipher, atbash-cipher, scytale-cipher)
for frequency-based attacks, and by ML crates for basic statistics.
Zero external dependencies.

## Usage

```rust
use stats::{mean, variance, chi_squared, index_of_coincidence};

let avg = mean(&[1.0, 2.0, 3.0, 4.0, 5.0]); // 3.0
let var = variance(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0], false); // 4.571...
let chi2 = chi_squared(&[10.0, 20.0, 30.0], &[20.0, 20.0, 20.0]); // 10.0
let ic = index_of_coincidence("AABB"); // 0.333...
```

## Module Structure

- `descriptive` -- mean, median, mode, variance, standard_deviation, min, max, range
- `frequency` -- frequency_count, frequency_distribution, chi_squared, chi_squared_text
- `cryptanalysis` -- index_of_coincidence, entropy, english_frequencies

## Spec

See `code/specs/ST01-stats.md` for the full interface contract.
