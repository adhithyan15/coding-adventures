# Microbenchmark (Kotlin)

Paired monotonic-clock collection, exact ratio analysis, and replay validation for workload labels that contain a non-NUL character outside ASCII whitespace (U+0020 and U+0009 through U+000D). Other Unicode whitespace, including U+2003, is allowed. The included CLI measures the indexed-array workload; replay validates report data without executing the named workload. JSON integer values preserve the signed 64-bit domain. The `microbenchmark-kt` artifact depends on the separate `rank-statistics-kt` artifact.

Run `gradle test installDist` and then `gradle run --args="--pairs 40 --revision local"`. This targets JDK 21; Gradle outputs go to `gradle-build/` to avoid the repository's required `BUILD` file. See [VER07d](../../../specs/VER07d-reference-microbenchmarks.md) and [VER07l](../../../specs/VER07l-jvm-microbenchmarks.md).

The Maven coordinate is `com.codingadventures:microbenchmark-kt:0.1.0`; publish
`com.codingadventures:rank-statistics-kt:0.1.0` first. The included local
composite build substitutes that sibling during development. No remote
registry publication is performed here.
