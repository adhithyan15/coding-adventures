# stats (Ruby)

Descriptive statistics, frequency analysis, and cryptanalysis helpers for the
coding-adventures monorepo.

## What It Does

This gem provides three modules under `CodingAdventures::Stats`:

1. **Descriptive** -- mean, median, mode, variance, standard_deviation,
   min, max, range.
2. **Frequency** -- frequency_count, frequency_distribution, chi_squared,
   chi_squared_text.
3. **Cryptanalysis** -- index_of_coincidence, entropy, ENGLISH_FREQUENCIES.

## Installation

```bash
bundle install
```

## Usage

```ruby
require "coding_adventures_stats"

CodingAdventures::Stats::Descriptive.mean([1, 2, 3, 4, 5])  # => 3.0
CodingAdventures::Stats::Frequency.frequency_count("Hello")  # => {"H"=>1, "E"=>1, "L"=>2, "O"=>1}
CodingAdventures::Stats::Cryptanalysis.index_of_coincidence("AABB")  # => 0.333...
```

## How It Fits

This is the ST01 package from the coding-adventures spec. It provides
reusable statistics for the CR (cipher) packages and future ML workloads.

## Running Tests

```bash
bundle exec rake test
```
