# Changelog

## 0.1.0

- Validate replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.

- Add paired collection, strict report replay, exact-rank analysis, and a native indexed-array command.
- Declare the local rank-statistics package prerequisite in both native BUILD entrypoints.
- Install package-declared test and development dependencies before invoking Dune.
