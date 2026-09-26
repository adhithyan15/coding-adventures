## HL-C445-6c9b9cfc — Letter-anchoring ratchets now have per-track owners

**Status: CLOSED — implemented by #16089.** The post-#16083 contention audit
counted only paths still tracked on current main and commits since 2026-09-25.
`tests/letter-anchoring.test.ts` was the largest remaining cross-track authoring
hotspot at 15 touches: seventeen script tracks shared one `CEILINGS` map, so a
track improving its own writing runway still edited an aggregate test fixture.

Each non-Latin track now owns one strict JSON ratchet for `cold`,
`buildsToward`, and `unwritten`. Discovery rejects missing, extra, malformed,
uppercase, case-fold-colliding, nested, and symlinked owners, while the corpus
test derives its exact track roster from the complete owner set.

The next measured contention point is the filmstrip aggregate set at 10 recent
touches per file: `generated-figure-hashes.json`, `filmstrip-geometry.json`,
`tests/figure-targets.test.ts`, and `tests/integration.test.ts`.
