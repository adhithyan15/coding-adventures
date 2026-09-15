# `iir-to-llvm`'s own real-execution write-barrier differential is NOT automatically portable from `field_store` to `array_set` — a within-one-frame vacuous pass, no callee-return boundary needed (AOT00-T8 follow-up)

Adding `array_set`'s generational write-barrier emission (mirroring `field_store`'s —
see `iir-to-llvm/src/lib.rs`'s `lower_array_set`/`lower_field_store` — and the
aarch64/x86_64 siblings' identical fix), I built a real-execution differential mirroring
`lang-aot/tests/llvm_gc_write_barrier.rs` (`field_store`'s own, which genuinely works —
confirmed via a `TEMP-REVERT-CHECK` that correctly turns the assertion red): allocate
`parent` (a 1-element `array<i64>`), minor-collect to tenure it old, allocate `child`,
`array_set parent, 0, child`, minor-collect again, assert `gc_live_bytes() == 32`
(both survived) via the barrier's remembered-set edge, with neither `parent` nor
`child` referenced by any local past the store — same shape as `field_store`'s test,
same reasoning for why nothing else should root either object into the second collect.

It passed — with the barrier call present, **and** with it fully deleted (no barrier
call, no address computed at all, i.e. behaving exactly like the code before this PR).
Unlike the aarch64 lesson directly above, this is **not** a callee-return-boundary
gap — everything happens in one `main` frame, no calls into a separate `IIRFunction`.
Root cause (probable, not fully isolated): `array_set`'s codegen is structurally
heavier than `field_store`'s — it emits `emit_bounds_check`'s conditional branch
(`icmp uge` + `br` to a trap block) in addition to the element `getelementptr`+`store`,
where `field_store` is straight-line with no branch at all. A branch forces `%handle`
to be available on both edges, which is exactly the kind of live-range shape an
unoptimized (`-O0`, no `opt` passes ever run on this hand-written IR) codegen path
spills to a stack slot rather than a register — and unlike a register, a stack slot
that nothing later reuses keeps its stale bit pattern for the rest of the frame,
conservatively rediscoverable by any later collect regardless of the write barrier.
Since `parent`'s own address is what leaks, and a **directly root-reachable** old
object is traced into on every collect regardless of remembered-set membership (see
`code/specs/AOT00-T9-moving-minor-collector.md` §3's identical point about
root/region-reachable old objects), `child` — sitting in `parent`'s own payload —
is found either way. This was verified by *removing* every barrier-adjacent
instruction (not just the call), so it is not an artifact of leftover dead-code
computed for the barrier itself; the pre-existing, unfixed `array_set` would be
equally vacuous under this exact test shape.

**Fix: don't ship the differential.** Deleted it rather than keep a passing-either-way
test in the suite (a false-positive regression guard is worse than no guard — it would
silently stop catching a reverted fix). Relied instead on: `iir-to-llvm`'s own
IR-string unit tests (`array_set_calls_the_generational_write_barrier` et al. in
`tests/test_backend.rs` — these DID catch a real bug during development: an earlier
draft passed `handle` itself as the barrier's `parent`, which is `raw_payload + 8`
in this backend's `alloc_array` lowering, not the true base `write_barrier` needs);
`field_store`'s own already-working differential, which proves the barrier
*mechanism* end-to-end for a call shape where it demonstrably matters; and
`gc-core`'s own generic, already-reviewed `write_barrier` tests. Exactly the same
call the aarch64 PR (lesson above) and `array_ref_tracing.rs` (`lang-aot` — "This
file does NOT attempt to prove the reclamation bug end-to-end… The actual, reliable
regression proof lives at the gc-core level instead") already made for structurally
similar reasons: **don't chase a real-execution differential across a scan gap this
codebase already knows is conservative-biased; the unit-level proof is the reliable
one.** If a real-execution proof for this specific op is ever needed again: forcing
`%handle`'s stack slot to be reused before the second collect (e.g. by doing enough
unrelated allocation/branching work in between) is the same "make it churn" approach
`gc-core-capi/src/stack_scan.rs`'s `DEAD_BATCH` fix above took, but wasn't pursued
here given the unit-level signal was already sufficient.
