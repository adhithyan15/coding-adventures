# Changelog

## 0.1.0

- Validate replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.

- Add paired collection, strict report replay, exact-rank analysis, and a native indexed-array command.
- Add pinned Bisect instrumentation: 91.42% library, 87.23% array-workload,
  and 83.67% CLI point coverage. Retain a native 40-pair report/replay check;
  the C clock shim is not covered by these OCaml metrics.
- Declare the local rank-statistics package prerequisite in both native BUILD entrypoints.
- Install package-declared test and development dependencies before invoking Dune.
