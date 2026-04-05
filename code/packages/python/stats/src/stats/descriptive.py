"""Descriptive statistics -- scalar functions operating on lists of floats.

Each function takes a list of numbers and returns a single float summary.
These are the building blocks of statistical analysis: they tell you about
the center (mean, median, mode), spread (variance, standard deviation, range),
and boundaries (min, max) of a dataset.

Worked Example
==============
Given the dataset [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]:

  mean     = (2+4+4+4+5+5+7+9) / 8 = 40 / 8 = 5.0
  median   = average of 4th and 5th values (sorted) = (4+5)/2 = 4.5
  mode     = 4.0 (appears 3 times, more than any other)
  variance = sample: sum of squared deviations / (n-1)
           = [(2-5)^2 + (4-5)^2 + ... + (9-5)^2] / 7
           = 32 / 7 = 4.571428...
  std_dev  = sqrt(4.571428...) = 2.138...
  min      = 2.0
  max      = 9.0
  range    = 9.0 - 2.0 = 7.0
"""

from __future__ import annotations

import math


def mean(values: list[float]) -> float:
    """Arithmetic mean: sum of all values divided by the count.

    The mean is the most common measure of central tendency. It uses every
    data point, which makes it sensitive to outliers. For example, the mean
    of [1, 2, 3, 100] is 26.5, even though most values are small.

    Formula: mean = (x_1 + x_2 + ... + x_n) / n

    >>> mean([1, 2, 3, 4, 5])
    3.0
    """
    if not values:
        msg = "mean requires at least one value"
        raise ValueError(msg)
    return sum(values) / len(values)


def median(values: list[float]) -> float:
    """Median: the middle value when sorted.

    The median splits the dataset in half -- 50% of values are below it and
    50% are above. Unlike the mean, the median is robust to outliers.

    For odd-length lists, the median is the middle element.
    For even-length lists, it is the average of the two middle elements.

    Examples:
      median([1, 2, 3, 4, 5])  -> 3.0   (middle of 5 elements)
      median([1, 2, 3, 4])     -> 2.5   (average of 2 and 3)
    """
    if not values:
        msg = "median requires at least one value"
        raise ValueError(msg)
    sorted_vals = sorted(values)
    n = len(sorted_vals)
    mid = n // 2
    # ── Odd length: single middle element ──
    if n % 2 == 1:
        return float(sorted_vals[mid])
    # ── Even length: average of two middle elements ──
    return (sorted_vals[mid - 1] + sorted_vals[mid]) / 2.0


def mode(values: list[float]) -> float:
    """Mode: the most frequently occurring value.

    If multiple values share the highest frequency, the one that appears
    first in the original list wins. This "first occurrence" tie-breaking
    rule ensures deterministic results across all languages in the repo.

    How it works:
    1. Count occurrences of each value.
    2. Find the maximum count.
    3. Return the first value in the original list that has that count.

    >>> mode([1, 2, 2, 3])
    2.0
    """
    if not values:
        msg = "mode requires at least one value"
        raise ValueError(msg)

    # ── Step 1: count occurrences ──
    counts: dict[float, int] = {}
    for v in values:
        counts[v] = counts.get(v, 0) + 1

    # ── Step 2: find the maximum frequency ──
    max_count = max(counts.values())

    # ── Step 3: return the first value with that frequency ──
    for v in values:
        if counts[v] == max_count:
            return float(v)

    # This line is unreachable but satisfies the type checker.
    return float(values[0])  # pragma: no cover


def variance(values: list[float], *, population: bool = False) -> float:
    """Variance: average of squared deviations from the mean.

    Variance measures how spread out the data is. A variance of 0 means
    all values are identical.

    Two flavors:
      - **Sample variance** (default, population=False): divides by n-1.
        Used when your data is a sample from a larger population. The n-1
        correction (Bessel's correction) makes the estimate unbiased.
      - **Population variance** (population=True): divides by n.
        Used when your data IS the entire population.

    Formula:
      variance = Sum((x_i - mean)^2) / d
      where d = n (population) or n-1 (sample)

    >>> variance([2, 4, 4, 4, 5, 5, 7, 9])          # sample
    4.571428571428571
    >>> variance([2, 4, 4, 4, 5, 5, 7, 9], population=True)
    4.0
    """
    if not values:
        msg = "variance requires at least one value"
        raise ValueError(msg)
    n = len(values)
    if n == 1 and not population:
        msg = "sample variance requires at least two values"
        raise ValueError(msg)

    m = mean(values)
    # ── Sum of squared deviations ──
    # Each (x_i - mean)^2 measures how far that point is from the center.
    squared_diffs = sum((x - m) ** 2 for x in values)

    divisor = n if population else (n - 1)
    return squared_diffs / divisor


def standard_deviation(values: list[float], *, population: bool = False) -> float:
    """Standard deviation: square root of variance.

    The standard deviation has the same units as the original data (unlike
    variance, which is in squared units). This makes it more interpretable.

    For a normal distribution:
      - ~68% of data falls within 1 standard deviation of the mean
      - ~95% falls within 2 standard deviations
      - ~99.7% falls within 3 standard deviations

    >>> standard_deviation([2, 4, 4, 4, 5, 5, 7, 9])  # sample
    2.138...
    """
    return math.sqrt(variance(values, population=population))


def min(values: list[float]) -> float:  # noqa: A001
    """Minimum value in the dataset.

    We shadow the built-in min() deliberately to provide a consistent
    interface. The built-in is used internally via the builtins module.

    >>> min([3, 1, 4, 1, 5])
    1.0
    """
    if not values:
        msg = "min requires at least one value"
        raise ValueError(msg)
    import builtins

    return float(builtins.min(values))


def max(values: list[float]) -> float:  # noqa: A001
    """Maximum value in the dataset.

    >>> max([3, 1, 4, 1, 5])
    5.0
    """
    if not values:
        msg = "max requires at least one value"
        raise ValueError(msg)
    import builtins

    return float(builtins.max(values))


def range(values: list[float]) -> float:  # noqa: A001
    """Range: the difference between the maximum and minimum values.

    The range is the simplest measure of spread. It only looks at the two
    extreme values, so it is very sensitive to outliers.

    Formula: range = max - min

    >>> range([2, 4, 4, 4, 5, 5, 7, 9])
    7.0
    """
    return max(values) - min(values)
