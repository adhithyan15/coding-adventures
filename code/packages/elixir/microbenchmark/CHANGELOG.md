# Changelog

## 0.1.0

- Validate replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.

- Add rank-statistics dependency, paired collection, replay, and indexed-array CLI.
- Use the high-resolution Windows performance counter with regression checks; smoke-test real positive timings by replaying the emitted report.
