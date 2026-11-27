# Changelog

## 0.1.0

- Count semantic JSON values before decoding replay input, including root and container values while excluding object keys.
- Scan replay scalars without copying attacker-controlled token bytes into temporary lists.
- Validate replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.

- Add rank-statistics dependency, paired collection, replay, and indexed-array CLI.
- Use the high-resolution Windows performance counter with regression checks; smoke-test real positive timings by replaying the emitted report.
- Bound public benchmark analysis at 10,000 pairs before mapping, and read CLI replay paths through a 4 MiB plus one byte limit.
- Align package compatibility with the repository's pinned Elixir 1.18.4 / OTP 27.3.4.11 runtime.
