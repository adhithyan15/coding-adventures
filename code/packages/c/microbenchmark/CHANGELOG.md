# Changelog

## 0.1.0

- Add paired microbenchmark collection, replay validation, and indexed-array CLI.
- Add injectable clocks and Windows/POSIX monotonic nanosecond adapters.
- Reject reports whose serialized analysis differs from recomputed values; expand
  malformed replay, allocator cleanup, and CLI file-boundary regressions.
- Give `MB_JSON` its own status name for actionable replay diagnostics.
- Accept portable core-only reports and complete producer-defined metadata,
  retaining duplicate, shape, bounds, and recomputed-analysis validation.
