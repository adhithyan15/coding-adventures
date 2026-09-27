## HL-C449-316f8f05 — Figure-target contracts now have concern-owned test shards

**Status: CLOSED — implemented by #16106.** After the Arabic corpus gate was
sharded, `tests/figure-targets.test.ts` remained the next shared test hotspot at
three recent touches. Candidate derivation, declared-target precedence, image
placement, the shared TeX macro, and real-corpus publication all lived in one
module even though those contracts change for different reasons.

The figure-target gate now discovers five deterministic concern-owned case
modules through the existing hardened test-shard loader. A shared synthetic
lesson fixture keeps the unit cases small, while the real-corpus owner still
loads the curriculum once and preserves its exact target-count and publication
assertions.

The same audit now has no unsharded cross-track test aggregate above two recent
touches. Remaining repeated files are already-sharded entrypoints, generated or
track-owned artifacts, and the two-touch `src/letter-anchoring.ts`; another
concurrency split should wait for a measured collision rather than inventing
one.
