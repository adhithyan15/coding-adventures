# Changelog

## Unreleased

- Fixed: the CBR01 portable-conformance test (all 55 fixture cases,
  round-tripped through decode/encode/error-path checks) was failing CI
  intermittently by exceeding vitest's 5s default per-test timeout on loaded
  runners -- it takes ~10s on its own, an order of magnitude more than every
  other test in the file. Gave it an explicit 15s timeout rather than raising
  the suite-wide default, since nothing else here needs the extra room.

## 0.1.0 - 2026-08-26

- Add the native TypeScript CBR01 value model, bounded checked encoder, and
  strict canonical decoder with no production dependencies.
- Preserve the full unsigned 64-bit domain with `bigint`, enforce deterministic
  map order, strict Unicode, portable depth/output limits, atomic append, and
  the closed payload-blind error taxonomy.
- Consume all 55 language-neutral vectors, add adversarial allocation and
  mutability coverage, and enforce a 90% production line-coverage gate.
