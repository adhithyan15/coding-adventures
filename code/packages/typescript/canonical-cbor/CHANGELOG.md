# Changelog

## Unreleased

- Fixed (root cause): the CBR01 conformance test still timed out at 15s
  under a full-repository CI build. Profiling showed the codec was never the
  cost: encoding the 1 MiB size-limit case takes ~70ms, and vitest's
  `toEqual` on two 1 MiB `Uint8Array`s took ~6.3s of the test's ~6.4s idle
  runtime. Encoded bytes are now compared with a linear `expectBytes` helper
  that checks the type, length and every byte, and names the first differing
  offset. The test now takes ~120ms. Mutation-checked: flipping one byte of
  the 1 MiB output, or appending a byte to every output, still fails with the
  offset or length named.
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
