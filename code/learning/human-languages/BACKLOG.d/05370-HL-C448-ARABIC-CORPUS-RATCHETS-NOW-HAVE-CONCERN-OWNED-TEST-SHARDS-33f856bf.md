## HL-C448-80e01d62 — Arabic corpus ratchets now have concern-owned test shards

**Status: CLOSED — implemented by #16101.** The post-integration contention
audit found `tests/corpus/arabic.test.ts` at three recent touches. Arabic
continuity, modality, lesson-content, writing-ramp, and root-ledger work all
edited that one track-local module even though those contracts advance in
independent curriculum tranches.

The Arabic corpus gate now discovers deterministic concern-owned case modules
through the existing corpus-test shard loader. Continuity and lesson-shape
budgets, the complete pre-A1 writing ramp, and the root ledger have separate
owners while preserving every assertion. The whole-corpus writing ratchet also
has an explicit timeout budget for its measured parse-and-render cost. Full CI
also exposed the independently owned integration-evidence gate crossing the
default timeout under suite load, so that real-corpus gate now has the same
explicit 60-second budget.

After excluding already-sharded aggregates and generated or track-local
artifacts, the next shared test hotspot is `tests/figure-targets.test.ts` at
three recent touches. It still combines filmstrip candidate rules, placement,
shared TeX macro contracts, and real-corpus publication checks in one module.
