# Stats (Perl)

Descriptive statistics, frequency analysis, and cryptanalysis helpers for the coding-adventures project.

## Overview

This package provides pure functions for:

1. **Descriptive statistics** -- mean, median, mode, variance, standard_deviation, stats_min, stats_max, stats_range
2. **Frequency analysis** -- frequency_count, frequency_distribution, chi_squared, chi_squared_text
3. **Cryptanalysis helpers** -- index_of_coincidence, entropy, ENGLISH_FREQUENCIES

## Usage

```perl
use CodingAdventures::Stats qw(
    mean median mode variance standard_deviation
    stats_min stats_max stats_range
    frequency_count frequency_distribution
    chi_squared chi_squared_text
    index_of_coincidence entropy
    ENGLISH_FREQUENCIES
);

# Descriptive statistics
print mean(1, 2, 3, 4, 5);           # 3.0
print median(2, 4, 4, 4, 5, 5, 7, 9); # 4.5
print variance([2, 4, 4, 4, 5, 5, 7, 9]); # 4.571... (sample)
print variance([2, 4, 4, 4, 5, 5, 7, 9], population => 1); # 4.0

# Frequency analysis
my $counts = frequency_count("Hello World");
print $counts->{L};  # 3

my $chi2 = chi_squared([10, 20, 30], [20, 20, 20]);
print $chi2;  # 10.0

# Cryptanalysis
my $ic = index_of_coincidence("AABB");
print $ic;  # 0.333...
```

## API Notes

- `stats_min`, `stats_max`, `stats_range` are prefixed with `stats_` to avoid collision with Perl built-in `min`/`max` from List::Util.
- `variance` and `standard_deviation` take an arrayref as the first argument, with optional `population => 1` named parameter.
- All other descriptive stats functions take a flat list of values.

## Running Tests

```sh
cpanm --installdeps --quiet .
prove -l -v t/
```
