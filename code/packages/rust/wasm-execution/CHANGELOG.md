# Changelog

All notable changes to this package will be documented in this file.

## [0.6.0] — 2026-08-03 (W04 — real garbage collection for `gc_heap`)

The WasmGC struct heap (`gc_heap`) was, until now, an append-only arena with
no reclamation — its own doc comment justified this as "bounded by the VM's
instruction budget," a budget that does not actually exist anywhere in this
crate. A long-running WASM-compiled program could allocate without bound.

- **New `gc` module**: a real mark-and-sweep collector over a **tombstone +
  free-list slot arena** — `gc_heap: Vec<GcStruct>` becomes
  `Vec<Option<GcStruct>>` (`Some` = live, `None` = a reclaimed slot ready
  for reuse). Compaction is out of scope: a `WasmValue::Ref(Some(handle))`
  is a `Vec` index (a WASM-spec-mandated representation), so shrinking the
  arena would silently invalidate every other live handle past the removed
  index. No generation tag is needed to guard a reused slot against
  aliasing a stale reference — mark-sweep's ordinary soundness argument
  (nothing left unmarked is reachable, given an exhaustive root walk)
  already rules that out, since every handle a program can hold either sat
  in a scanned root or was freshly minted.
- **Root set**: every `WasmValue::Ref(Some(_))` in `ctx.globals`,
  `ctx.typed_locals`, **every** `ctx.saved_frames[*].locals` (a paused
  caller's locals — missing this would free something a suspended caller
  still references), and the interpreter's operand stack
  (`GenericVM::typed_stack`, via the existing `REF_TAG` convention),
  traced transitively through each object's own fields — cycle-safe by
  construction (a worklist walk with a `marked` visited set, not naive
  recursion). Precise, with no `HeapKind`-style schema needed: a
  `GcStruct` field is a tagged `WasmValue`, self-describing as a reference
  or not.
- **Checked at two chokepoints**: `execute_branch` (every taken
  `br`/`br_if`/`br_table` — a loop's back-edge is a branch to its own loop
  label) and the internal `call_function` helper (every `call`/
  `call_indirect`, nested or not) — both of this crate's independent
  dispatch loops (`GenericVM::execute_with_context` and the hand-inlined
  loop inside `call_function`) route through these two shared functions, so
  instrumenting them covers both loops without adding anything WASM-specific
  to the generic `virtual-machine` crate's own dispatch code. Mirrors the
  existing "safepoints at back-edges and calls" convention rather than a
  per-instruction counter.
- **Adaptive object-count threshold**, mirroring `gc-core::FlatHeap::should_collect`/
  `adapt_threshold` verbatim (just in units of live objects rather than
  bytes, since this heap has no byte-size concept). New `gc-core` path
  dependency, reusing its representation-agnostic `GcProfile`/`GcCycleStats`
  for diagnostic consistency with the native-AOT and `vm-core` GC paths —
  exposed via new `WasmExecutionEngine::gc_live_object_count()` /
  `gc_profile()` accessors (`gc_heap` itself still resets every call, as it
  always has; only the counters are written back).
- Tests: `gc`'s own unit tests (`mark`/`sweep`/`alloc`/threshold-adaptation
  in isolation, including cycle-safety and a saved-caller-frame root) plus
  `end_to_end_loop_reclaims_garbage_and_preserves_kept_object` — a real WASM
  loop, driven through actual bytecode dispatch, that allocates 2000
  objects while keeping exactly one alive, proving both that the kept
  object survives with its field intact and that the live count is
  reclaimed mid-run rather than left to accumulate.
- **Security-review fixes, before this landed**:
  - `sweep` now also drops `gc_heap`'s trailing run of tombstoned slots
    (removing their indices from `free_list` too). Without this, the
    arena's length — and therefore `mark`/`sweep`'s O(len) cost — was a
    monotonically non-decreasing high-water mark for the life of a call: a
    program that transiently spiked the live count high, then settled into
    low-retention churn, would keep paying that peak cost on every
    subsequent collection. Not compaction (no live object moves or is
    renumbered) — only provably-all-garbage trailing capacity is dropped.
  - `gc::alloc`'s new-handle assignment now uses a checked `u32` conversion
    (clean trap on failure) instead of an unchecked `as u32` cast, so an
    eventual overflow (practically unreachable — it implies 100+GB of
    live, uncollected heap) can't silently alias a fresh object's handle
    onto an already-occupied lower index.
  - Tests: `sweep_shrinks_arena_when_everything_is_reclaimed`,
    `sweep_does_not_truncate_past_a_live_object_in_the_middle`.
- See `code/specs/W04-wasm-gc.md` for the full design rationale.

## [0.5.0] — 2026-07-07

### Added — `memory.copy` bulk-memory op (E4-dyn runtime string concat)

The engine now runs the `memory.copy` bulk-memory instruction, which
`iir-to-wasm` emits for runtime `str_concat` (splicing two operands' bytes into a
freshly allocated `[i32 len][bytes]` block).

- **Decoder**: a new `0xFC` two-byte-prefix branch (mirroring the `0xFB` GC prefix)
  reads the sub-opcode and its memory-index immediates, carrying the sub-opcode in a
  plain `Int` operand. `memory.copy` (`0x0A`, two index bytes) and `memory.fill`
  (`0x0B`, one index byte) are decoded; only `memory.copy` is executed.
- **Execution**: a `0xFC` handler pops `size`, `src`, `dest` and delegates to the new
  `LinearMemory::copy`, which bounds-checks both ranges (either out of range traps —
  never panics) and uses `copy_within` for overlap-safe (memmove) semantics. A
  zero-length copy is a no-op even at the end of memory.
- Tests: `memory_copy_decodes_as_one_0xfc_instruction`,
  `linear_memory_copy_moves_bytes_overlap_safe`.

## [0.4.0] — 2026-06-08

### Added — `ref.test` execution (LANG77 / McCarthy L3b-3a-4)

The engine now runs the WasmGC **`ref.test`** type-test op — the primitive
McCarthy `pair?` lowers to ("is this lisp value a cons cell?"):

- The decoder reads the heap-type immediate of `ref.test` (`0xFB 0x14`) and the
  nullable `ref.test null` (`0xFB 0x15`).
- The engine executes them: pop a reference, push `i32 1` if it is a (non-null)
  struct reference, else `0`. Our value model has exactly one struct type
  (`$LispyPair`), so a `struct.new` result — the only `Ref(Some(_))` — is that
  type; a boxed integer (`i31` payload / `I32`) or the null reference yields `0`.
  The `0x15` variant additionally accepts the null reference.

So `pair?(cons)` → 1, `pair?(atom)` → 0, `pair?(nil)` → 0. 3 new tests.

## [0.3.0] — 2026-06-04

### Added — WasmGC struct heap + references (LANG77 / McCarthy L3b-3a-3b)

The engine now *runs* the WasmGC object opcodes, so a lisp **cons cell**
(`$LispyPair`) can be allocated, read, and mutated in-repo — `(CAR (CONS 7 9))`
executes to `7` on the engine.

- **`WasmValue::Ref(Option<u32>)`** — a new value variant for GC references:
  `None` is the null reference (`ref.null` / lisp `nil`), `Some(handle)` indexes
  the engine's GC object heap. Round-trips through the typed stack tagged as
  `anyref` (`0x6E`). (An `i31ref` stays an `I32` payload, as in 0.2.0.)
- **GC object heap** — an append-only arena of `GcStruct { type_idx, fields }`
  on the execution context. `struct.new` allocates and returns a handle; the
  heap persists across calls within a run (a cons built in a callee survives).
  No reclamation: total allocations are bounded by the VM instruction budget.
- **Opcodes executed:**
  - `struct.new <type>` (`0xFB 0x00`) — pops the registered field count of
    values, allocates a `GcStruct`, pushes `Ref(Some(handle))`.
  - `struct.get <type> <field>` (`0xFB 0x02`) — reads a field of a non-null ref.
  - `struct.set <type> <field>` (`0xFB 0x04`) — writes a field of a non-null ref.
  - `ref.null` (`0xD0 0x0F`) — pushes the null reference.
  - `ref.is_null` (`0xD1`) — pops an anyref, pushes `1`/`0`.
- **Decoder** — the `0xFB` block now reads the struct ops' index immediates
  (type/field) into a `Gc { sub, type_idx, field_idx }` operand; `0xD0` consumes
  its one-byte heap-type immediate so it isn't mis-decoded.
- **`WasmExecutionEngine::set_struct_field_counts`** — registers struct type
  field counts (the parser doesn't yet surface struct types to the engine; this
  is populated from the parsed module in L3b-3a-3c).
- **Defaults** — nullable reference locals (`anyref`, `structref`) now default
  to `Ref(None)`; `i31ref` stays `I32(0)`.

Every failure mode (unknown type/field index, null dereference, missing arity,
type mismatch, unknown sub-opcode) is a **clean trap**, never a panic. 10 new
tests (cons/car/cdr round-trips, `struct.set` mutation, `ref.is_null`, and the
null-deref / out-of-range / missing-arity traps).

Also removed a pre-existing dead no-op branch in `decode_immediates`.

## [0.2.0] — 2026-06-05

### Added — WasmGC `i31` execution (LANG77 / McCarthy L3b-3a-3a)

First step of executing WebAssembly **GC** opcodes (so the McCarthy-Lisp → wasm
value model can be run in-repo, not just emitted):

- **`decode_function_body`** now decodes the two-byte `0xFB` GC prefix: it reads
  the sub-opcode byte and carries it in the instruction's operand (the MVP
  opcode table is single-byte and doesn't know `0xFB`). Previously a `0xFB`
  stream would be mis-decoded as separate single-byte instructions.
- The engine executes the `i31` boxing pair: **`i31.new`** (`0xFB 0x1C`) and
  **`i31.get_s`** (`0xFB 0x1D`). An `i31ref` is represented as its plain `i32`
  payload on the value stack (the small lisp integers we box never need the
  reference identity), so both are stack-identity no-ops — the integer passes
  straight through. `i32.const 42 → i31.new → i31.get_s` returns `42`.
- Unimplemented GC sub-opcodes (`struct.*`, `ref.*`) are a clean `Err`, not a
  panic — they land with the GC-object-heap slice (L3b-3a-3b).

3 new tests: the two-byte decode, the i31 box/unbox round-trip executed on the
engine, and the unsupported-opcode clean error.

## [0.1.1] — 2026-05-13

### Fixed

- **`br_table` handler** — corrected out-of-range index handling: when the
  branch index exceeds the target table length, the default label is now
  used (as per the WASM spec).  Previously the handler indexed past the
  end of the table, causing a panic.
- **`call_indirect` stub** — added a graceful error message for
  `call_indirect` (not yet supported) so the engine reports the opcode name
  instead of panicking on an unknown byte.
- **`i32.const` sign extension** — `i32.const` operands are now
  LEB128-decoded as signed (i32) and zero-extended to i64, matching the WASM
  spec for negative literal values.
- Various opcode stubs improved to match the WASM binary format byte layout.

## [0.1.0] - 2026-04-05

### Added

- Initial package scaffolding generated by scaffold-generator
