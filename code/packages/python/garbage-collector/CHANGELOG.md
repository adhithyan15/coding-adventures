# Changelog

## [Unreleased]

### Added — LANG16 PR2: dispatch-hook methods on the GarbageCollector ABC

Two new methods, both with conservative no-op defaults so existing
implementations (`MarkAndSweepGC`) continue to work unchanged:

- `should_collect() -> bool` — vm-core consults this at every safepoint
  (after any instruction whose `IIRInstr.may_alloc` is True, plus the
  periodic forced-safepoint interval).  Returns `True` by default
  (collect at every safepoint) — that's the simplest correct policy
  and the dispatcher's safepoint interval bounds the overhead.
  Generational / incremental / concurrent collectors will override
  this to defer collections until policy-specific triggers fire.
- `write_barrier(parent_address, child_address) -> None` — vm-core
  invokes this from the `field_store` opcode handler whenever a heap
  reference is stored into another heap object.  Default is a no-op;
  generational collectors override for remembered sets, tri-color
  for grey-on-write invariants, refcount for incref/decref tracking.

These fill in the seams LANG16 PR3 (vm-core integration) needs at
GC safepoints and at `field_store`.

### Test coverage

`MarkAndSweepGC` inherits both defaults — verified by tests that show
heap state and stats are unaffected by `write_barrier` calls and that
`should_collect` returns the conservative True.  A `_RecordingGC`
exercises the override path: a flip-flopping `should_collect` and a
recorded list of (parent, child) `write_barrier` pairs.  44 tests
pass at 96% line coverage.

## 0.1.0 — 2026-03-20

### Added

- **GarbageCollector ABC** — pluggable interface for any GC algorithm
- **HeapObject types**: `ConsCell`, `Symbol`, `LispClosure` with `references()` methods
- **MarkAndSweepGC** — McCarthy's 1960 algorithm: mark reachable objects, sweep the rest
- **SymbolTable** — symbol interning with GC-managed heap allocation
- `is_valid_address()` for safe address checking
- `stats()` for introspection (total allocations, collections, freed, heap size)
- Root scanning supports nested lists, dicts, and HeapObject references
- Heap addresses start at 0x10000 to avoid ambiguity with small integer values
- 44 tests (32 mark-sweep + 12 symbol table), 96% coverage
- Full literate programming documentation
