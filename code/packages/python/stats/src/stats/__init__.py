"""stats -- Descriptive statistics, frequency analysis, and cryptanalysis helpers.

Overview
========
This package provides three categories of pure functions:

1. **Descriptive statistics** (mean, median, mode, variance, standard deviation,
   min, max, range) -- operate on arrays of floats.
2. **Frequency analysis** (frequency_count, frequency_distribution, chi_squared,
   chi_squared_text) -- operate on text strings or parallel arrays.
3. **Cryptanalysis helpers** (index_of_coincidence, entropy, ENGLISH_FREQUENCIES)
   -- tools for breaking classical ciphers.

Design Principles
=================
- **Pure functions.** No side effects, no mutation of inputs.
- **No external dependencies.** Pure math only.
- **Population vs sample.** Variance and standard deviation default to sample
  (Bessel-corrected, dividing by n-1). Pass population=True for population
  statistics (dividing by n).
- **Tree-shakeable.** Each function lives in its own module so you can import
  only what you need.
"""

# ── Descriptive statistics ──────────────────────────────────────────────
from stats.descriptive import (
    max as max,
    mean as mean,
    median as median,
    min as min,
    mode as mode,
    range as range,
    standard_deviation as standard_deviation,
    variance as variance,
)

# ── Frequency analysis ──────────────────────────────────────────────────
from stats.frequency import (
    chi_squared as chi_squared,
    chi_squared_text as chi_squared_text,
    frequency_count as frequency_count,
    frequency_distribution as frequency_distribution,
)

# ── Cryptanalysis helpers ───────────────────────────────────────────────
from stats.cryptanalysis import (
    ENGLISH_FREQUENCIES as ENGLISH_FREQUENCIES,
    entropy as entropy,
    index_of_coincidence as index_of_coincidence,
)

__all__ = [
    # Descriptive
    "mean",
    "median",
    "mode",
    "variance",
    "standard_deviation",
    "min",
    "max",
    "range",
    # Frequency
    "frequency_count",
    "frequency_distribution",
    "chi_squared",
    "chi_squared_text",
    # Cryptanalysis
    "index_of_coincidence",
    "entropy",
    "ENGLISH_FREQUENCIES",
]
