# Dart microbenchmark

Pure Dart paired batch collector, analyzer, exact signed-64-bit JSON replay, and indexed-array command. The command constructs its 4,096-element `BigInt` array before collection and both callbacks traverse that exact caller-owned list. Use `Report.fromJson` to validate and replay exported observations.

Run `BUILD` or `BUILD_windows` for VM and Node tests and the native 40-pair smoke. The sibling rank-statistics package is a local monorepo dependency; this lane does not claim a hosted registry release.
