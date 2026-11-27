# Changelog

## 0.1.0

- Define blank workload IDs by the six ASCII whitespace characters, accept other Unicode IDs, and reject NULs or malformed surrogate escapes.

- Add injectable paired collection, strict report replay, and exact analysis.
- Add the indexed-array CLI with monotonic wall timing and signed-64-bit JSON.
- Preserve full binary64 precision in analysis fields so emitted reports replay exactly.
- Add independent oracle, wide-cutoff, fake-clock, replay, and workload tests.
- Validate caller-supplied array elements as signed 64-bit integers in both traversal helpers and use exact checked addition.
- Bound JSON token density before allocating the report AST, while retaining quote-aware strings and the 10,000-pair limit.
- Use a POSIX test path in the Unix BUILD entrypoint so the test file is found under `sh -c`.
- Accept producer-supplied nonblank workload identifiers during strict report replay; the array CLI retains its own workload ID.
- Reject benchmark analysis above 10,000 pairs before pair inspection and read CLI replay paths with a bounded 4 MiB plus one byte.
- Declare the monotonic-time, named revision, output, and replay-file runtime effects in the capability profile.
