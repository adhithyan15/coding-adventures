# W16 — multi-memory: enough of the proposal to pass `memory_grow.wast`

## Purpose

Logged as task #85 during the post-W15 prioritization scan: after task #86
(v128 invoke arguments) merged, the full 61-file vendored corpus reached
100% on every `assert_*` directive kind except `module` (933/934) and
`register` (1/2) — both caused by the exact same root cause, confirmed by
direct inspection of the checked-in baseline
(`code/packages/rust/wasm-conformance/tests/fixtures/testsuite-status.json`):
`memory_grow.wast` declares a module with more than one linear memory,
which this interpreter has never supported (`wasm-validator` hard-rejects
any module with more than 1 total memory, a real WASM 1.0 MVP rule this
repo enforces correctly per spec — multi-memory is its own, later,
Phase-2 proposal, not part of MVP).

This is the last remaining conformance gap in the entire vendored corpus.
Task #85's own prior logged description explicitly warned against
reflexively re-grading the resulting `TooManyMemories`/`InvalidDataSegment`
errors as `NotYetSupported` — that would mask a real, currently-correct
structural-validation rule (the W14 design principle: structural failures
stay `Fail` on purpose). The actual fix is to implement enough of the real
multi-memory proposal to make `memory_grow.wast` instantiate and run
correctly, then let the validator's rule become "at most `MAX_MEMORIES`"
instead of "at most 1" — not to hide the gap.

## Scope: narrower than the full multi-memory proposal, confirmed by direct inspection of the one file that needs it

`code/packages/rust/wasm-conformance/tests/fixtures/testsuite/memory_grow.wast`
declares multiple `(memory ...)` forms (imported and local, e.g. `$mem1`
imported with limits `1 6`, `$mem2` imported with no max, `$mem3`/`$mem4`
locally declared) and exercises them **exclusively** through
`memory.size $mem1` / `memory.grow $mem1 (...)`-style instructions with a
named memory operand. Confirmed by direct `grep` across the fixture: it
contains **zero** `i32.load`/`i64.store`/etc. instructions with an explicit
non-default memory operand, **zero** `memory.copy`/`memory.fill`/
`memory.init`/`data.drop` directives, and its one `(data ...)` segment
targets the default (index-0) memory implicitly.

This means the real upstream proposal's hardest piece — repurposing bit
`0x40` of a `memarg`'s LEB128-encoded `align` byte as a "memory index
follows" flag for every load/store instruction — is **not** needed to
close this specific gap, and is explicitly **out of scope** for this spec.
So are giving `memory.copy`/`memory.fill` real multi-memory-aware source/
destination indices (they already have index bytes reserved in the decoder
per WASM 1.0's own "must be zero" forward-compatibility rule; multi-memory
would just lift that restriction, but nothing in the vendored corpus
exercises it yet) and implementing `memory.init`/`data.drop` at all (both
are entirely unimplemented today, independent of multi-memory — a
separate, pre-existing gap, not created or worsened by this spec).

## The concrete problem, confirmed by direct inspection

### `wasm-types`/`wasm-module-parser`/`wasm-wast-parser`: already fully general, no changes needed

- `WasmModule.memories: Vec<MemoryType>` (`code/packages/rust/wasm-types/
  src/lib.rs:837`) already holds arbitrarily many memories; `MemoryType`
  (lib.rs:476-486) carries no arity constraint itself — the WASM-1.0-only
  doc comment on it (lib.rs:462) is descriptive prose, not an enforced
  invariant.
- `DataSegment.memory_index: u32` (wasm-types/src/lib.rs:709) is already a
  real field, not hardcoded to 0.
- The binary decoder already parses N memories generically: `parse_memory_
  section` loops and pushes every entry (wasm-module-parser/src/lib.rs:
  702-708); `parse_data_section` reads a real per-segment `memory_index`
  LEB128 off the wire (lib.rs:854-865); import parsing handles
  `ExternalKind::Memory` generically inside its loop (lib.rs:629-632), so
  multiple memory imports already decode correctly today.
- The text decoder's two-pass memory declaration already supports N
  `(memory ...)` forms and already resolves named indices for `(data
  (memory $name) ...)` via the existing `ctx.memory_names: HashMap<String,
  u32>` (wasm-wast-parser/src/module.rs:165, used at :1055-1057) — this
  machinery already works for every construct except the two instructions
  below.

### `wasm-wast-parser`: `memory.size`/`memory.grow` hardcode the memory-index byte to 0 and never consume a `$name` token

Both instruction-encoding paths — the bare-atom/stream form and the
folded/flat form (the one `memory_grow.wast` actually uses) — share the
identical bug, confirmed at both sites:

```rust
// code/packages/rust/wasm-wast-parser/src/module.rs:1455-1459 (stream form)
"memory.size" | "memory.grow" => {
    out.push(info.opcode);
    out.push(0x00);
    Ok(0)
}

// code/packages/rust/wasm-wast-parser/src/module.rs:1791-1797 (folded form)
"memory.size" | "memory.grow" => {
    let (operands, _) = split_operands_and_immediates(args, 0);
    encode_instr_list(operands, icx, out)?;
    out.push(info.opcode);
    out.push(0x00); // memory index, always 0
    Ok(())
}
```

`split_operands_and_immediates(args, 0)` declares **zero** immediates for
these two instructions, so a leading `$mem1` atom in `(memory.grow $mem1
(local.get 0))` is treated as an *operand* and recursed into
`encode_instr_list` as if it were itself a nested instruction — which then
fails opcode lookup on `$mem1`. This is confirmed as the actual, literal,
currently-observed parse failure for this exact file (not a validator
rejection — the parse never gets that far): the checked-in golden baseline
records
`"memory_grow.wast": "at byte 266: unknown instruction \"$mem1\""`
(`code/packages/rust/wasm-conformance/tests/fixtures/testsuite-status.json`),
and byte 266 of the fixture is the `$mem1` token inside the file's
`memory.size $mem1` instruction.

### `wasm-validator`: hardcoded to "at most 1 memory total" and "data segment memory index must be 0"

```rust
// code/packages/rust/wasm-validator/src/lib.rs:169-178
let total_memories = imported_memories + module.memories.len();
if total_memories > 1 {
    return Err(ValidationError::TooManyMemories(format!(
        "WASM 1.0 allows at most 1 memory, found {} ({} imported + {} declared)",
        total_memories, imported_memories, module.memories.len()
    )));
}
```

and, separately (lib.rs:282-296), every data segment's `seg.memory_index !=
0` is rejected outright with `InvalidDataSegment`. Both are real, correct
WASM-1.0-MVP rules today — this spec changes the first cap's numeric bound
(still bounded, not unbounded) and generalizes the second check to bounds-
check against `total_memories` instead of hardcoding zero.

### `wasm-execution`: memory is a single `Option`, and `memory.size`/`memory.grow`'s already-decoded memory-index operand is discarded

- `WasmExecutionContext.memory: Option<*mut LinearMemory>` (wasm-execution/
  src/lib.rs:1504); `WasmEngineConfig.memory`/`WasmEngineState.memory`
  (both `Option<LinearMemory>`, lib.rs:4535/4546); `WasmExecutionEngine.
  memory: Option<Box<LinearMemory>>` (lib.rs:4564); `get_memory()` (lib.rs:
  1924-1929) always resolves the single field or errors.
- `wasm-opcodes` already declares a `memidx` immediate for both opcodes
  (`code/packages/rust/wasm-opcodes/src/lib.rs:335-336`), and the generic
  operand decoder already has a `"memidx"` decode arm that reads it off
  the wire — but the actual handlers ignore the decoded value entirely and
  always touch the single `ctx.memory`:
  ```rust
  // code/packages/rust/wasm-execution/src/lib.rs:3590-3599 (memory.size, 0x3F)
  vm.register_context_opcode(0x3F, |vm, _instr, _code, ctx| {
      let ctx = get_ctx(ctx);
      let size = match ctx.memory {
          Some(ptr) => unsafe { (*ptr).size() as i32 },
          None => 0,
      };
      ...
  });
  // code/packages/rust/wasm-execution/src/lib.rs:3602-3612 (memory.grow, 0x40)
  vm.register_context_opcode(0x40, |vm, _instr, _code, ctx| {
      let ctx = get_ctx(ctx);
      let delta = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
      let result = match ctx.memory {
          Some(ptr) => unsafe { (*ptr).grow(delta as u32) },
          None => -1,
      };
      ...
  });
  ```
  Both handlers take `_instr` (underscore-prefixed, unused) — the memidx
  byte is decoded generically upstream and never reaches here at all.

### `wasm-runtime`: `WasmInstance.memory` is singular; import resolution overwrites rather than accumulates; instantiation only ever allocates `memories[0]`

- `WasmInstance.memory: Option<LinearMemory>` (wasm-runtime/src/lib.rs:
  1084-1088, the "Allocated linear memory" field on the real instance
  struct — distinct from the unrelated `WasiEnv.memory: Arc<Mutex<
  Option<LinearMemory>>>` at lib.rs:377, a WASI host-import plumbing detail
  that is not part of this spec).
- Import resolution **overwrites** rather than accumulates: each
  `ImportTypeInfo::Memory` match sets `memory = Some(imported_mem)`
  unconditionally (lib.rs:1221-1231) — a module importing two memories
  would silently keep only the second.
- Allocation is **hardcoded to `module.memories[0]`**, ignoring any
  further declared memories:
  ```rust
  // code/packages/rust/wasm-runtime/src/lib.rs:1275-1278
  if memory.is_none() && !module.memories.is_empty() {
      let mem_type = &module.memories[0];
      memory = Some(LinearMemory::new(mem_type.limits.min, mem_type.limits.max));
  }
  ```
- Data-segment application always targets the single `memory`, ignoring
  `seg.memory_index` (lib.rs:1305-1312) — fine for this spec's scope since
  `memory_grow.wast`'s one data segment targets the default memory, but
  worth stating precisely: this spec does not make data-segment
  application memory-index-aware; see "What does NOT change" below.
- `build_engine` (lib.rs:1504-1519) currently `.take()`s the single
  `instance.memory` (a move, not a clone — unlike `globals`/`v128_heap`,
  which round-trip via clone because nothing about calling a function
  needs exclusive memory ownership the way table/memory mutation does);
  `call_engine`/`call_engine_with_v128` both restore `instance.memory =
  state.memory;` unconditionally, even on a trapped call (lib.rs:1609,
  1633) — this take/restore shape is preserved as-is, just widened from a
  single value to a `Vec`.

## Design

### `MAX_MEMORIES`: a real, bounded cap — not unbounded, not left at 1

Per real-world WebAssembly engines and the multi-memory proposal itself,
there is no spec-mandated numeric ceiling on memory count (unlike MVP's
hardcoded 1) — concrete limits are implementation-defined. This repo
already has a same-shape precedent for "generous but bounded, to prevent a
malicious/malformed module from causing unbounded allocation before any
real work runs": `MAX_CALL_DEPTH = 1200` (wasm-execution/src/lib.rs:1673),
`MAX_V128_HEAP_LEN = 1_000_000` (lib.rs:1720, `pub` since task #86). This
spec adds `pub const MAX_MEMORIES: usize = 64` next to those two in
`wasm-execution` (well above `memory_grow.wast`'s real count of 4, and
above any plausible legitimate module, while still bounding a
maliciously-crafted module's memory-section entry count before allocation
runs) and re-exports it for `wasm-validator` to enforce, mirroring
`MAX_V128_HEAP_LEN`'s existing cross-crate reuse pattern rather than
picking a second, independently-chosen number.

`wasm-validator/src/lib.rs:169-178`'s check becomes:
```rust
let total_memories = imported_memories + module.memories.len();
if total_memories > wasm_execution::MAX_MEMORIES {
    return Err(ValidationError::TooManyMemories(format!(
        "at most {} memories allowed, found {} ({} imported + {} declared)",
        wasm_execution::MAX_MEMORIES, total_memories, imported_memories, module.memories.len()
    )));
}
```
(`ValidationError::TooManyMemories`'s variant and message stay generically
worded — "too many", not "more than 1" — so this reads correctly regardless
of where the cap is set.)

The data-segment check (lib.rs:282-296) changes from "must equal 0" to
"must be `< total_memories`" — the identical bounds-check shape already
used for every other index-space validation in this file (function
indices, table indices, global indices), not a new pattern.

### `wasm-wast-parser`: resolve a leading memory-index token for `memory.size`/`memory.grow`

Both arms (module.rs:1455-1459 and :1791-1797) change from an
unconditional `out.push(0x00)` to resolving an optional leading `$name`/
numeric-literal token via the existing `ctx.memory_names`/`resolve_idx`
machinery already used by `(data (memory $name) ...)`, defaulting to index
0 when no token is present (preserving every existing single-memory
`.wast`/`.wat` fixture's encoding byte-for-byte). The folded form's
`split_operands_and_immediates(args, 0)` call becomes `split_operands_and_
immediates(args, 1)` so a leading memory-index token is correctly
classified as an immediate rather than recursed into as an operand,
matching the exact shape `"call" | "return_call"` already uses for their
own leading `func`-index immediate (module.rs:1722-1728) — not a new
pattern, the fourth instance of an existing one.

### `wasm-execution`: memory becomes a `Vec`, and `memory.size`/`memory.grow` read their already-decoded index

- `WasmExecutionContext.memory: Option<*mut LinearMemory>` →
  `memories: Vec<*mut LinearMemory>`; `WasmEngineConfig.memory`/
  `WasmEngineState.memory`/`WasmExecutionEngine.memory` (`Option<...>`) →
  `memories: Vec<LinearMemory>`/`Vec<Box<LinearMemory>>` respectively.
- `get_memory()` (lib.rs:1924-1929) gains a `memidx: usize` parameter,
  bounds-checking against `ctx.memories.len()` and returning a targeted
  `VMError` (e.g. `"memory index N out of range (M memories)"`) on
  overflow rather than the current unconditional "no memory available".
  Every existing load/store/bulk-memory call site passes `0` explicitly
  (matching WASM 1.0's own "index must be 0" encoding for those
  instructions today — this spec does not decode a real index for them,
  see "What does NOT change"), so this is a mechanical signature change
  at every call site, not a behavior change for anything except
  `memory.size`/`memory.grow`.
- The `0x3F`/`0x40` handlers stop taking `_instr` (underscore-prefixed,
  unused) and instead read the already-decoded `memidx` operand (the
  `"memidx"` arm already present in the generic decoder, lib.rs:1244-1252
  region) to select which `Vec` entry to size/grow, exactly the same
  "decoded but discarded" plumbing that already exists — this activates
  it, it does not add it.

### `wasm-runtime`: `WasmInstance.memory` becomes `Vec<LinearMemory>`; import resolution accumulates; instantiation allocates every declared memory

- `WasmInstance.memory: Option<LinearMemory>` → `memories: Vec<LinearMemory>`.
- Import resolution (lib.rs:1221-1231) changes from `memory = Some(...)`
  (overwrite) to `memories.push(...)` (accumulate) — imported memories
  fill the low indices, matching the same import-then-declared ordering
  every other index space (functions, tables, globals) already uses in
  this same loop.
- Allocation (lib.rs:1275-1278) changes from the single hardcoded
  `module.memories[0]` read to a loop over every entry in
  `module.memories`, pushing a newly-allocated `LinearMemory` for each.
- `build_engine`/`call_engine`/`call_engine_with_v128` (lib.rs:1504-1519,
  1609, 1633): `instance.memory.take()` → `std::mem::take(&mut
  instance.memories)` (already the exact pattern used for `instance.
  tables` two lines below the current memory line, lib.rs:1507 — this
  spec just applies the same existing shape to memory too); the
  unconditional-even-on-trap restore semantics are unchanged, only the
  type widens.
- Data-segment application (lib.rs:1305-1312) stays targeting a single
  memory (index 0) for this spec's scope — see "What does NOT change".

## What does NOT change

- **Load/store instructions** (`i32.load`, `i64.store`, etc.) — no
  `memarg` align-byte high-bit decoding, no non-zero-memory-index support.
  Every load/store call site is updated mechanically to pass memory index
  `0` to the widened `get_memory()`, preserving exact current behavior for
  every existing test and every vendored corpus file (`memory_grow.wast`
  itself never issues a load/store against a non-default memory).
- **`memory.copy`/`memory.fill`** — their reserved memory-index decode
  bytes (already skipped-but-unread today, wasm-execution/src/lib.rs:1057-
  1058) stay unread; both keep operating on memory index 0 only.
- **`memory.init`/`data.drop`** — remain entirely unimplemented, exactly
  as they are today; this spec neither adds nor worsens that gap.
- **Data-segment application during instantiation** — stays targeting
  memory 0 regardless of `seg.memory_index`'s now-generalized validation
  bound; `memory_grow.wast`'s one data segment targets the default memory,
  so this does not block this spec's goal, but is a real, named scope
  boundary for the next slice of the proposal to pick up.
- `wasm-conformance`'s `Executor`/directive-grading logic — no code
  changes; only a baseline regen once the fix lands.

## Staged commits

1. This spec (sign-off only).
2. Implementation, in dependency order: `wasm-execution` (`MAX_MEMORIES`
   constant, `Vec`-ify every memory field/config/state struct, widen
   `get_memory()` with a bounds-checked `memidx` param + every load/store/
   bulk-memory call site passing `0`, activate the `0x3F`/`0x40` handlers'
   already-decoded memidx operand) → `wasm-runtime` (`WasmInstance.
   memories: Vec<LinearMemory>`, accumulate-not-overwrite import
   resolution, allocate every declared memory, `mem::take` plumbing) →
   `wasm-validator` (generalize both checks against `MAX_MEMORIES`/
   `total_memories`) → `wasm-wast-parser` (resolve `memory.size`/
   `memory.grow`'s leading memory-index token in both encoding paths).
   New tests: a module with 2+ memories instantiates and validates; each
   memory's `size`/`grow` targets the correct, independent `LinearMemory`
   (grow one, confirm the other's size is unchanged); a module exceeding
   `MAX_MEMORIES` is still correctly rejected; a TEMP-REVERT-CHECK proving
   the `memory.size $mem2`-targets-the-right-memory case is load-bearing
   (reverting the handler's memidx read reproduces the predicted
   "always targets memory 0" wrong-answer bug, not a crash — the subtler,
   more dangerous failure mode this spec exists to prevent).
3. Baseline regen against `memory_grow.wast` and the rest of the 61-file
   vendored corpus, confirming `module` reaches 934/934 and `register`
   reaches 2/2 with zero regressions anywhere else.

## Verification

- `wasm-execution`/`wasm-runtime` unit tests: a hand-built 2-memory
  instance's `memory.size $mem0`/`memory.size $mem1` (or their `memidx`-
  parameterized equivalents) return independently correct sizes; growing
  one memory does not change the other's size; a 3rd-memory `data`
  segment's now-generalized bounds check accepts a valid index and still
  rejects an out-of-range one.
- A hand-built module with `MAX_MEMORIES + 1` memories is still rejected
  by `wasm-validator` with `TooManyMemories` (proving the cap moved, not
  disappeared).
- TEMP-REVERT-CHECK on the `memory.size`/`memory.grow` memidx-read fix
  (see Staged Commits #2) confirms it is load-bearing.
- `wasm-conformance` baseline regen: `memory_grow.wast` moves from a
  file-level parse failure to a fully passing file; aggregate `module`
  reaches 934/934 (100%) and `register` reaches 2/2 (100%); full before/
  after diff of every file's per-directive-kind tally confirms zero
  regressions anywhere else in the 61-file vendored corpus, matching this
  session's established verification discipline.
- `cargo test -p wasm-types -p wasm-module-parser -p wasm-wast-parser -p
  wasm-validator -p wasm-execution -p wasm-runtime -p wasm-conformance`
  and `cargo clippy` clean across every touched crate.
