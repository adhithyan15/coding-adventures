# Rank statistics

`coding-adventures-rank-statistics` provides a sample median and an exact,
distribution-free confidence interval for a population median. Inputs are
finite real observations; the functions sort a copy and preserve ties.

```python
from rank_statistics import median, median_interval

median([1.0, 4.0, 2.0, 3.0])  # 2.5
median_interval([1, 2, 3, 4, 5, 6], 75)  # (2.0, 5.0)
```

`median_interval` accepts integer confidence percentages from 1 through 99
and returns endpoint observations or `None` when the sample is too small.
This package contains no benchmark timing or practical threshold policy.
