# Dart microbenchmark

Pure Dart paired batch collector, analyzer, exact signed-64-bit JSON replay, and indexed-array command. The command constructs its 4,096-element `BigInt` array before collection and both callbacks traverse that exact caller-owned list. Seeded pair orders are reproducible within this lane; seed `0` uses a fixed nonzero internal xorshift state because zero is absorbing, while the reported seed remains `0`. `Report.fromJson` accepts finite JSON numbers for `minimum_factor` in integer or decimal notation and validates and replays exported observations.

Run `BUILD` or `BUILD_windows` for VM and Node tests and the native 40-pair smoke. The sibling rank-statistics package is a local monorepo dependency; this lane does not claim a hosted registry release.

Benchmark analysis accepts at most 10,000 pairs; the separate rank-statistics
API has no sample-count cap. Replay paths are read through a 4 MiB plus one
byte boundary before UTF-8 decoding.
