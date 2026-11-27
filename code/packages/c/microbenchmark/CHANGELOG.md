# Changelog

## 0.1.0

- Reject workload IDs containing only the six ASCII whitespace characters;
  require other descriptive metadata to be nonempty and NUL-free without
  assigning it an extra whitespace policy.
- Add paired microbenchmark collection, replay validation, and indexed-array CLI.
- Add injectable clocks and Windows/POSIX monotonic nanosecond adapters.
- Reject reports whose serialized analysis differs from recomputed values; expand
  malformed replay, allocator cleanup, and CLI file-boundary regressions.
- Give `MB_JSON` its own status name for actionable replay diagnostics.
- Accept portable core-only reports and complete producer-defined metadata,
  retaining duplicate, shape, bounds, and recomputed-analysis validation.
- Capture expected native replay failures as process results in Windows
  PowerShell, then assert their exit status and output explicitly.
- Build, install, and run the installed consumer with one consistent CMake
  configuration across single- and multi-config generators.
- Resolve the installed consumer from the configuration-specific output folder
  when a POSIX multi-config generator creates it there, with single-config and
  Windows executable suffix handling.
