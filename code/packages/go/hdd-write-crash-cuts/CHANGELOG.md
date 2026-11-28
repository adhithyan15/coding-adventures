# Changelog

## Unreleased

- Add a pure HDD sector-write crash-cut transition with exact byte-prefix
  splicing, durable-prefix evidence, acknowledgement-only receipts, detached
  images, and ordered typed refusals.
- Add strict bounded test-only replay of the shared DB04a vector, an independent
  byte-array oracle, the full 320-outcome cut sweep, seeded and fuzz histories,
  malformed-input checks, and seven clean-controlled semantic mutants.
- Add `Describe` to report a cut's persisted-byte count and acknowledgement
  using the same admission path as `Apply`, without inspecting media or
  allocating.
- Cross all cut kinds with native-int prefix boundaries and verify `Describe`
  parity with `Apply`, zero allocations, and two additional clean-controlled
  descriptor mutations.
