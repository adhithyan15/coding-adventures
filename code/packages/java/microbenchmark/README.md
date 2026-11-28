# Microbenchmark (Java)

Paired monotonic-clock collection and replayable JSON reports. The included CLI measures the indexed-array workload; the library can validate portable reports for a workload label that contains a non-NUL character outside ASCII whitespace (U+0020 and U+0009 through U+000D) without executing it. Other Unicode whitespace, including U+2003, is allowed. The analysis library depends on the separate `rank-statistics` artifact. JSON integer fields remain exact signed 64-bit values; benchmark speed is advisory on shared machines.

```sh
gradle test installDist
gradle run --args="--pairs 40 --revision local"
```

For a saved report, run `gradle run --args="--replay path/to/report.json"`. This package targets JDK 21. Gradle output is redirected to `gradle-build/` to avoid colliding with this repository's `BUILD` script. See [`VER07d`](../../../specs/VER07d-reference-microbenchmarks.md) and [`VER07l`](../../../specs/VER07l-jvm-microbenchmarks.md).

The Maven coordinate is `com.codingadventures:microbenchmark:0.1.0`; publish
`com.codingadventures:rank-statistics:0.1.0` first. Gradle's local composite
build substitutes that sibling during development. This change does not
publish either artifact to a remote registry.
