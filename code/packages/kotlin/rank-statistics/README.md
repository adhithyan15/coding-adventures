# Rank statistics (Kotlin)

Finite-sample median and exact distribution-free confidence interval over finite `Double` samples. Inputs are copied before sorting; `BigInteger` arithmetic decides the order-statistic cutoff.

```kotlin
val median = RankStatistics.median(doubleArrayOf(1.0, 2.0, 3.0, 4.0))
val interval = RankStatistics.medianInterval(samples, 95)
```

Run `gradle test` with JDK 21. Gradle output uses `gradle-build/` to avoid the required `BUILD` script. See [VER07c](../../../specs/VER07c-rank-statistics.md) and [VER07l](../../../specs/VER07l-jvm-microbenchmarks.md).

The Maven publication coordinate is `com.codingadventures:rank-statistics-kt:0.1.0`.
