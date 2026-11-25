# Rank statistics (Java)

Reusable sample median and exact median confidence interval for finite binary64 samples. Inputs are copied before sorting. The exact rank cutoff uses `BigInteger`; it does not rely on rounded floating point probabilities.

```java
double median = RankStatistics.median(new double[] {1, 2, 3, 4});
RankStatistics.Interval ci = RankStatistics.medianInterval(samples, 95);
```

Use JDK 21 and `gradle test`. Gradle output goes to `gradle-build/` to avoid colliding with this repository's required `BUILD` script. See [`VER07c`](../../../specs/VER07c-rank-statistics.md) and [`VER07l`](../../../specs/VER07l-jvm-microbenchmarks.md).

The Maven publication coordinate is `com.codingadventures:rank-statistics:0.1.0`.
