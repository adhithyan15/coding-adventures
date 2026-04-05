# stats (Python)

Descriptive statistics, frequency analysis, and cryptanalysis helpers for the
coding-adventures monorepo.

## What It Does

This package provides three categories of pure functions:

1. **Descriptive statistics** -- mean, median, mode, variance, standard
   deviation, min, max, range.
2. **Frequency analysis** -- frequency_count, frequency_distribution,
   chi_squared, chi_squared_text.
3. **Cryptanalysis helpers** -- index_of_coincidence, entropy, plus a
   standard English letter frequency table.

## Installation

```bash
uv pip install -e ".[dev]"
```

## Usage

```python
from stats import mean, median, variance, frequency_count, index_of_coincidence

# Descriptive statistics
mean([1, 2, 3, 4, 5])          # 3.0
median([1, 2, 3, 4])           # 2.5
variance([2, 4, 4, 4, 5, 5, 7, 9])  # 4.571... (sample)

# Frequency analysis
frequency_count("Hello")       # {'H': 1, 'E': 1, 'L': 2, 'O': 1}

# Cryptanalysis
index_of_coincidence("AABB")   # 0.333...
```

## How It Fits

This is the ST01 package from the coding-adventures spec. It provides
reusable statistics for the CR (cipher) packages and future ML workloads.
The chi-squared and IC functions were extracted from the CR00 Caesar cipher
implementation for cross-package reuse.

## Running Tests

```bash
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```
