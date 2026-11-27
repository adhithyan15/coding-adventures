# Changelog

## Unreleased

- Instrument the library and CLI with pinned Bisect PPX and require per-file coverage for the OCaml source boundaries.
- Validate complete report-schema mutations and every public analysis, clock, and workload failure phase.
- Format the native driver, workload, library interface, implementation, and
  tests with the pinned OCamlFormat 0.27.0 so the package's Dune `@fmt` gate
  passes in an isolated build.

## 0.1.0

- Declare the monotonic-clock FFI and time effects plus the OCaml external
  exception; the capability gate pins this manifest and the native shim.
- Validate replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.
- Preserve full-report schema mutations and send malformed Unicode probes directly to the production decoder, without test-side decoding.

- Add paired collection, strict report replay, exact-rank analysis, and a native indexed-array command.
- Add pinned Bisect instrumentation: 92.32% library after workload-boundary validation, 87.23% array-workload,
  and 83.67% CLI point coverage. Retain a native 40-pair report/replay check;
  the C clock shim is not covered by these OCaml metrics.
- Declare the local rank-statistics package prerequisite in both native BUILD entrypoints.
- Install package-declared test and development dependencies before invoking Dune.
