# Dart rank-statistics

Pure Dart sample medians and exact distribution-free confidence intervals for a population median. `median` preserves its input and handles finite extreme values without midpoint overflow. `medianInterval` uses `BigInt` cutoff arithmetic and returns input order statistics.

Run `BUILD` or `BUILD_windows` for formatting, analysis, 95%+ line coverage,
and VM and Node tests. Tests replay the package-local copy of the canonical
paired-observation fixture; Node uses a generated snapshot so all cases run
without file I/O.
