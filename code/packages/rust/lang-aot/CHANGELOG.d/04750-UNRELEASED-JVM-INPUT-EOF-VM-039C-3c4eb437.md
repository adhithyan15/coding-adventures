## Unreleased — JVM input/EOF (VM-039c)

Support `input_more` through `env.BasicRuntime.inputMore()J`, preserving the
i64 result width. The matrix host shares a pushback stream between numeric
reads, string reads and non-consuming EOF peeks. Four FLOW-MATIC JVM cells
cover streams, empty input, partial records and repeated EOF. Java host tests
verify stable peeks, permissive parsing and propagation of I/O failures.

