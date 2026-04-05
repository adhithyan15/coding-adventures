# CodingAdventures.Stats

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

Used by cipher packages (Caesar, Atbash, Scytale) for frequency-based
attacks, and by ML packages for basic statistics. Zero external
dependencies.

## Usage

```elixir
alias CodingAdventures.Stats.{Descriptive, Frequency, Cryptanalysis}

Descriptive.mean([1, 2, 3, 4, 5])           # => 3.0
Descriptive.variance([2, 4, 4, 4, 5, 5, 7, 9]) # => 4.571... (sample)
Descriptive.variance([2, 4, 4, 4, 5, 5, 7, 9], population: true) # => 4.0

Frequency.chi_squared([10, 20, 30], [20, 20, 20]) # => 10.0

Cryptanalysis.index_of_coincidence("AABB") # => 0.333...
```

## Module Structure

- `CodingAdventures.Stats.Descriptive` -- mean, median, mode, variance, etc.
- `CodingAdventures.Stats.Frequency` -- frequency_count, frequency_distribution, chi_squared
- `CodingAdventures.Stats.Cryptanalysis` -- index_of_coincidence, entropy, english_frequencies

## Spec

See `code/specs/ST01-stats.md` for the full interface contract.
