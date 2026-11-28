# DB05d2b1 — Plan copies for a readable retiring NAND block

Extend the reviewed Go evacuation planner with a pure planning call
at `a9a6616bbb9f` for readable retirement. Use the admitted
page-map snapshot as the source of live logical ownership. The caller establishes that the
target is readable and has entered DB05d2a draining.

```go
func PlanDrain(table ftlpagemap.Table, pagesPerBlock, block int64,
    excludedBlocks []bool) (Plan, error)
```

Reuse the existing Plan and Copy representations and the common
bounded enumeration. Validate configured table dimensions, block
width, target index, exclusion-mask length and a pending reservation
in the existing order. Treat the new call's excluded blocks as
destination exclusions. Its source may already be excluded by the
retirement ledger. Plan every live source in increasing physical
order, pairing it with the lowest eligible free destination outside
the source extent.

An all-live target and an empty target are valid. An empty target
returns a fully located plan with an empty owned copy list. A short
spare reserve returns the existing typed capacity refusal and a zero
plan. Each scheduled destination must be distinct. The call returns
owned observations and checks no erase counter. The current
reclamation operation preserves its stale-page and target-mask
validation and its exact existing results.

Use the admitted allocation ledger at `aae49579bfb` and the
erase budget at `8af9585c616` in a downstream ledger integration test.
Go package build dependencies follow each module's `go.mod`.
Keep the planner's sole dependency on the page map so the Go resolver
can order the planner before the erase budget and ledger. Mark the
readable source draining and show that an all-live block with zero
erase budget generates a valid plan. Keep other factory-bad,
draining and retired blocks outside the destination pool. Compare
metadata to the existing allocator and established reserved-slot
precedence. Controller commands that execute the moves belong to the
next retirement slice.

Replay canonical model-first vectors. Close small map state graphs
and exhaust their destination masks, legal widths and target blocks.
Check exact capacity, one-short capacity, signed bounds, empty
targets, free but excluded slots, detached copy lists, and immutable
predecessors. Use the package's native coverage, race and vet, fixed
fixture/history fuzz budgets and cross-architecture checks. Exercise
unchanged reclamation tests with the shared enumeration. Prove
targeted mistakes for requiring stale pages, planning into a
second fenced block and reversing copy order.
