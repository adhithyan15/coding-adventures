# W25 — memory64 proposal, first slice: 64-bit memory addressing for scalar load/store

## Purpose and how this slice was chosen

Three consecutive prior sessions (`W20`, `W23`, `W24`) each independently
re-confirmed that non-null concrete reference types (`(ref $t)`, needed by
`call_ref` and the rest of the GC-continuation epic) are not separable into
a smaller first slice — every sub-piece needs the same new type-system
surface plus a new runtime value representation plus `call_ref`'s own
indirect-call machinery, all at once. This session re-read `W20`, `W23`,
and `W24` and is treating that finding as settled — see `W24`'s own
"Re-deriving the non-null-concrete-ref-type question directly" section for
the live, corpus-grounded evidence. This spec does not re-litigate it a
fourth time.

Per `W24`'s own sequencing, the next candidate is `memory64` — but `W23`
(the session that first scoped it) concluded: "No corpus file targets
memory64 in the pinned `WebAssembly/testsuite` tree at all (it lives in a
separate, not-yet-merged proposal repo at this SHA)... Clearly larger AND
lower-value than cross-instance tag identity this round." `W24` repeated
this conclusion without independently re-fetching the tree ("memory64
remains unscoped by ANY corpus coverage... confirmed unchanged"). Per this
campaign's own discipline (never trust a prior finding that was only
single-sourced), this session re-fetched the pinned tree directly:

```
gh api repos/WebAssembly/testsuite/git/trees/28864811cf03bdbf880733786148feaba339582d --jq '.tree[].path'
```

**This claim was wrong.** The pinned tree contains `memory64.wast` (7930
bytes), `memory64-imports.wast` (6728 bytes), `table64.wast`, and a whole
family of `*64.wast` files duplicating existing test files for the 64-bit
addressing case (`address64.wast`, `align64.wast`, `bulk64.wast`,
`endianness64.wast`, `load64.wast`, `memory_copy64.wast`,
`memory_fill64.wast`, `memory_grow64.wast`, `memory_init64.wast`,
`memory_redundancy64.wast`, `memory_trap64.wast`, `call_indirect64.wast`,
`table_copy64.wast`, `table_fill64.wast`, `table_get64.wast`,
`table_grow64.wast`, `table_init64.wast`, `table_set64.wast`,
`table_size64.wast`, `binary_leb128_64.wast`). `memory64` is not a
separate, not-yet-merged proposal at this SHA — it is part of the same
core testsuite every other file in this campaign already vendors from.
Real, vendorable, substantial conformance value is available here, right
now, at the exact same pinned commit. (`W23`'s effort-sizing observations —
that this is NOT additive the way tag identity was, that every load/store/
`memory.copy`/`memory.fill`/`memory.init` handler computes its address as
`u32` today — remain accurate and are confirmed independently by this
session's own reading of `wasm-execution::register_memory`. Only the
"no corpus" premise was false.)

### Why this is the pick over the GC continuation wall

`call_ref`/non-null-concrete-refs is genuinely blocked (three independent
sessions, no smaller sub-slice found). `memory64` is large but NOT blocked
— it is additive to the existing decode-a-byte-stream architecture (no new
`ValueType` variant, no new `WasmValue` representation: addresses just
become `i64` instead of `i32` for memories that opt in). Per this
campaign's own `W07` sizing convention this is an **L**-sized epic (dozens
of mechanical call sites across 5 crates), but it is a real, corpus-backed,
currently-blocked win, unlike the GC wall.

### Scoping the first slice: which files are actually separable

Read both real `.wast` files directly (fetched via `gh api .../git/blobs/
<sha>`, not inferred) before deciding what to vendor:

- **`memory64.wast`** (the file this slice targets): a single self-
  contained module declaring a `(memory i64 1)`, a data section, and a
  battery of `i32.load`/`i64.load`/`f32.load`/`f64.load` (plus their 8/16/
  32-bit sign/zero-extending variants) and `i32.store`/`i64.store`/
  `f32.store`/`f64.store` (plus narrowing variants) functions addressed
  with `i64.const` operands instead of `i32.const`. Also exercises
  `memory.size` returning `i64` for a 64-bit memory, a handful of
  `assert_invalid` module-validity checks (`size minimum must not be
  greater than maximum`, the `2^48`-page spec ceiling for a 64-bit memory's
  `min`/`max`, `unknown memory` when no memory is declared at all), and one
  `(module definition (memory i64 0x1_0000_0000_0000))` — a spec-VALID
  declaration this interpreter deliberately does not attempt to actually
  allocate (see "Practical allocation cap" below). Confirmed by reading the
  entire file: **no** table declarations, no SIMD, no atomics, no bulk
  memory ops (`memory.copy`/`memory.fill`/`memory.init`), no `memory.grow`
  invocation (only an `assert_invalid` case using the opcode name, which
  never reaches address-width logic because the "no memory declared"
  check fires first). This is a clean, self-contained slice: memory
  declarations, scalar load/store address width, `memory.size` result
  width, and data-segment offset width — nothing else.
- **`memory64-imports.wast`**: read in full. Roughly two-thirds of it is
  `table64` (`(table i64 10 funcref)`, `(table $tab64 (import ...) i64 10
  funcref)`) — a **different, separate** proposal from memory64 that
  extends tables to a 64-bit index space. Genuinely entangled: this file
  cannot be vendored cleanly without also building table64 support.
  **Deliberately deferred to a future slice** (see "Out of scope" below),
  exactly the same "vendor only the clean file, defer the entangled one"
  call `W20` made for `i31.wast` vs. `struct.wast`/`array.wast`.
- The other `*64.wast` files (`address64`, `align64`, `bulk64`,
  `endianness64`, `load64`, `memory_copy64`, `memory_fill64`,
  `memory_grow64`, `memory_init64`, `memory_redundancy64`,
  `memory_trap64`) were not read in full this session (time-boxed) but are
  known by name to need `memory.copy`/`memory.fill`/`memory.init`/
  `memory.grow` widened to 64-bit addressing, which this slice does not
  attempt for the bulk-memory family (it only widens plain load/store).
  Deferred to a future slice for the same reason.

## Scope

### In scope

1. **`wasm_types::Limits`**: `min`/`max` widened from `u32`/`Option<u32>`
   to `u64`/`Option<u64>` — required because a real, spec-valid 64-bit
   memory's `min`/`max` can be as large as `2^48` (`i31.wast`-style hex
   literal `0x1_0000_0000_0000`), which doesn't fit `u32`. Shared with
   `TableType` (this repo's `Limits` struct is table/memory-agnostic); all
   existing table values stay comfortably in range, so this is a pure
   widening, non-breaking numerically for every existing caller.
2. **`wasm_types::MemoryType` gains `pub is64: bool`** (default `false` at
   every existing call site — mechanical, no behavior change for any
   32-bit memory). `TableType` does **not** gain an equivalent field this
   slice — `table64` is explicitly out of scope (see below).
3. **`wasm-module-parser`/`wasm-module-encoder`**: `parse_limits`/
   `encode_limits` widened to recognize flags bits `0x04` (64-bit index,
   no max) / `0x05` (64-bit index, with max) per the real spec's binary
   `limits` grammar (verified live against `https://webassembly.github.io/
   spec/core/binary/types.html`, not assumed from memory: `0x00`/`0x01` =
   32-bit, `0x04`/`0x05` = 64-bit, `min`/`max` LEB128-encoded as `u64` when
   the `is64` bit is set). A table's limits flags byte with bit `0x04` set
   is rejected with a clear parse error (table64 out of scope, not
   silently misinterpreted).
4. **`wasm-wast-parser`**: text-form `(memory i64 <min> <max>?)` (and the
   inline-import desugared form) recognized — an `i64` keyword atom
   immediately after any optional `$name`, before the limit numbers, sets
   `is64`. New `numeric::parse_u64` (mirrors `parse_u32` exactly, using the
   same `parse_int_magnitude` u128 core already shared with `parse_i32`/
   `parse_i64`) used for a 64-bit memory's limit literals. The text
   parser's `memarg` `offset=`/`align=` attribute parsing (`parse_memarg`)
   is left `u32`-typed this slice — deliberately: `memory64.wast` never
   writes an explicit `offset=` literal outside `u32`'s range (LEB128
   bytes for a small value are identical regardless of whether the
   encoder that emitted them was reasoning in `u32` or `u64` terms), and
   widening this specific text-literal parser has no real corpus payoff
   yet. The real spec's binary `memarg` grammar was still re-verified live
   this session (`offset` is `u64` unconditionally in the wire format,
   regardless of the target memory's index type — verified against
   `https://webassembly.github.io/spec/core/binary/instructions.html`),
   which is why the DECODE side (`wasm-validator`/`wasm-execution`, below)
   widens to `u64` even though the text encoder does not yet need to
   produce values that require it. This repo's existing multi-memory
   memidx-in-align-bit-6 encoding, W18, is unaffected and reused as-is.
5. **`wasm-validator`**:
   - Per-memory `is64` tracked alongside the existing `memory_count`/
     `has_memory` context (same combined-index-space ordering: imports
     first, then declared, that `memory_count` itself already uses).
   - Load/store (`0x28..=0x3E`) pop `I64` instead of `I32` for the address
     operand when the target `memidx`'s memory `is64`; `memory.size`/
     `memory.grow` push/pop `I64` instead of `I32` under the same
     condition (`memory.grow`'s delta argument is `i64` too).
   - `memarg`'s `offset` immediate decoded as `u64` (widened from the
     existing `u32` LEB read), matching the wast-parser and execution
     changes.
   - **New**: `min <= max` is now actually checked for both memories and
     tables (a real, pre-existing gap — this repo had NO such check before
     this slice, for either 32-bit or 64-bit limits; found while chasing
     `memory64.wast`'s own `"size minimum must not be greater than
     maximum"` `assert_invalid` case, which nothing in this repo currently
     rejects for ANY memory, 32- or 64-bit).
   - Check 1b (the existing 65536-page spec-ceiling + DoS-aggregate check)
     made `is64`-aware: the spec ceiling for a 64-bit memory's `min`/`max`
     is `2^48` pages, not `2^16` — verified against `memory64.wast`'s own
     real `assert_invalid` boundary (`0x1_0000_0000_0001` invalid,
     `0x1_0000_0000_0000` valid). The DoS-motivated AGGREGATE aspect of
     Check 1b stays separate from this (see next point) — validation
     itself never allocates, so a large-but-spec-valid `is64` declaration
     is correctly accepted here even though this interpreter will refuse
     to actually instantiate one that large (next point).
6. **`wasm-execution`**:
   - `LinearMemory` gains `is64: bool` (set from the owning `MemoryType`
     at instantiation).
   - The shared `effective_addr` helper and every one of the ~23 scalar
     load/store opcode handlers (`i32.load`...`i64.store32`, 0x28-0x3E)
     branch on the target memory's `is64` to pop `WasmValue::I64` instead
     of `WasmValue::I32` for the address operand, computing the same
     wrapping `usize` effective address either way (an `i64` address is
     used as its full 64-bit bit pattern; a real memory's actual allocated
     size — capped well below `2^64` by the practical cap below — makes
     the existing `usize` bounds-check in `bounds_check` correct
     unchanged).
   - `memory.size`/`memory.grow` push/pop `WasmValue::I64` instead of
     `WasmValue::I32` under the same `is64` condition.
   - `DecodedOperand::MemArg.offset` widened `u32` → `u64`; the shared
     `memarg` immediate decoder reads it via a new `decode_leb_u64`
     (mirrors `decode_leb_u32`).
   - **New `MAX_MEMORY64_INITIAL_PAGES` practical instantiation cap**
     (see "Practical allocation cap" below) — a genuinely new DoS
     consideration `is64` introduces that the existing 32-bit path never
     needed (32-bit's own spec ceiling, `2^16` pages / 4 GiB, already
     doubles as a safe practical cap; `is64`'s spec ceiling, `2^48` pages,
     does not: `2^48 * 65536 = 2^64` bytes, which overflows a 64-bit
     multiplication and would abort the process via Rust's allocator
     error handler if ever actually attempted, not a catchable panic).
7. **`wasm-runtime`**: `instantiate()`'s data-segment offset evaluation
   (`evaluate_const_expr(...).as_i32()`, hardcoded) branches on memory 0's
   `is64` to call `.as_i64()` instead, matching the wast-parser emitting
   `i64.const` offset expressions for a 64-bit memory's active data
   segments. `LinearMemory::new`'s call site enforces the new practical
   cap, returning a real `TrapError` (not a panic/abort) if exceeded —
   `Directive::Module`'s existing `Err(e) => DirectiveOutcome::Trap(...)`
   arm (`wasm-conformance`) already handles this gracefully; confirmed by
   reading that match arm directly, not assumed.
8. **Vendor `memory64.wast` verbatim** (pinned SHA
   `28864811cf03bdbf880733786148feaba339582d`) into `wasm-conformance`, add
   to `TESTSUITE_FILES`, regenerate the baseline.

### Practical allocation cap: a genuinely new DoS consideration

Unlike every prior 32-bit-only slice, `is64`'s own spec-legal declaration
range (`min`/`max` up to `2^48` pages) is **far** larger than any real
system could actually back with allocated memory, and it overflows a
64-bit byte-count multiplication outright at its own ceiling
(`2^48 pages * 2^16 bytes/page = 2^64 bytes`). This repo's OWN
`memory.wast` already vendors `(module definition (memory 65536))` — a
real 4 GiB allocation attempt this repo already tolerates today for a
32-bit memory (65536 pages IS the entire 32-bit spec ceiling, so no
separate practical cap was ever needed there). `memory64.wast` introduces
`(module definition (memory i64 0x1_0000_0000_0000))` — spec-valid to
*declare*, but `2^48` pages is not something any real implementation
(including the actual production engines this spec targets) ever backs
with real allocated bytes; the spec itself treats memory limits above an
implementation's own practical ceiling as an implementation-defined
resource-limit trap, not a conformance failure, the same way this repo's
own `MAX_TABLE_ELEMENTS`/`MAX_V128_HEAP_LEN` are implementation-defined
resource limits on top of the spec's own more permissive syntax range.
`MAX_MEMORY64_INITIAL_PAGES` is set to `65536` (the same 4 GiB practical
ceiling 32-bit memories already live under) — generous relative to every
value any `assert_return`/`invoke` test in `memory64.wast` actually
instantiates (all use 1-2 pages), and it turns the one `2^48`-page bare
`(module definition ...)` directive into a graceful, honest `Trap` outcome
instead of aborting the whole conformance-test process.

### Explicitly out of scope (this slice — steps 2+ of this arc)

- **`table64`** (`(table i64 ...)`) — genuinely entangled with
  `memory64-imports.wast` (see "Scoping" above); a separate proposal, its
  own `Limits`/`TableType` widening, its own binary-flags-byte handling.
  Deferred; `memory64-imports.wast` deferred alongside it.
- **Bulk memory ops on a 64-bit memory** (`memory.copy`/`memory.fill`/
  `memory.init`, and the `*64.wast` files that exercise them:
  `bulk64.wast`, `memory_copy64.wast`, `memory_fill64.wast`,
  `memory_init64.wast`, `memory_redundancy64.wast`, `memory_trap64.wast`).
  `wasm-execution`'s bulk-memory handlers (a handful of dedicated
  functions, not the shared `effective_addr` this slice widens) need
  their own, separate `is64`-aware address-width branch.
  `memory_grow64.wast`/`address64.wast`/`align64.wast`/`load64.wast`/
  `endianness64.wast`/`binary_leb128_64.wast` are the same "duplicate of
  an existing file, but for a 64-bit memory" shape as `memory64.wast`
  itself and are likely cheap, mechanical follow-ons once this slice's
  plumbing exists — deferred only for session time-boxing, not because
  they're independently hard.
- **SIMD loads/stores (`v128.load*`/`v128.store*`) and atomic memory ops
  on a 64-bit memory** — separate helper functions
  (`wasm-execution`'s SIMD-specific `effective_addr(offset_imm: u32, base:
  i32)` at a different call-site family, and the `0xFE`-prefixed atomic
  family) that this slice does not touch. No vendored corpus file in this
  slice exercises either combination.
- **`call_indirect64.wast`** — needs table64 (a `call_indirect` against a
  64-bit-indexed table), not memory64 proper.
- **Non-null concrete reference types / `call_ref`** — unchanged from
  `W20`/`W23`/`W24`; not re-litigated here.

## Verification plan

- **Unit tests**:
  - `wasm-types`: `Limits`/`MemoryType` construction with `is64: true` and
    `u64`-range `min`/`max` values that don't fit `u32`.
  - `wasm-module-parser`/`wasm-module-encoder`: round-trip a `flags=0x04`/
    `0x05` memory limits encoding; a table with `flags=0x04` is rejected.
  - `wasm-wast-parser`: `(memory i64 1)`, `(memory i64 1 2)`,
    `(memory $m i64 1)` all parse with `is64: true`; a plain `(memory 1)`
    stays `is64: false`; `numeric::parse_u64` accepts `0x1_0000_0000_0000`
    and rejects `0x1_0000_0000_0000_0000_0000` (overflow) and negative
    literals.
  - `wasm-validator`: load/store on an `is64` memory require an `I64`
    address (an `I32` address is now a type error); `memory.size`/
    `memory.grow` on an `is64` memory produce/consume `I64`; the new
    `min <= max` check rejects `(memory 1 0)` and `(memory i64 1 0)`; the
    `is64`-aware Check 1b accepts `2^48` and rejects `2^48 + 1` for an
    `is64` memory's `min`/`max`.
  - `wasm-execution`: a scalar load/store round-trip through an `is64`
    memory using an `I64` address; `memory.size` returns `I64` for an
    `is64` memory; the new `decode_leb_u64` decodes a multi-byte memarg
    offset correctly; `MAX_MEMORY64_INITIAL_PAGES` rejected with a
    `TrapError` (not a panic) for an over-cap `is64` memory instantiation
    attempt.
  - `wasm-runtime`: an active data segment on an `is64` memory using an
    `i64.const` offset expression initializes the correct bytes;
    `limits_compatible` accounts for the widened `u64` fields.
- **Real corpus, measured** (`cargo run --bin wasm_conformance_report`,
  full JSON diff against the pre-change baseline, confirming only
  `memory64.wast` (new) moved): report the exact `module`/`assert_return`/
  `assert_invalid` counts achieved for `memory64.wast`, and confirm zero
  regressions anywhere else in the corpus.
- **Downstream consumers**: `cargo test -p lang-aot` and every other crate
  depending on `wasm-types`/`wasm-module-parser`/`wasm-module-encoder`/
  `wasm-wast-parser`/`wasm-validator`/`wasm-execution`/`wasm-runtime`
  (`wasm-conformance`, `twig-to-wasm`, `nib-wasm-compiler`,
  `brainfuck-wasm-compiler`, `ir-to-wasm-compiler`, `ir-to-wasm-validator`,
  `iir-to-wasm`) built/tested, since `wasm_types::Limits`/`MemoryType` is
  foundational and widely depended on (a prior PR in this campaign broke a
  downstream consumer silently by skipping this check).
- `/security-review` before push (the practical allocation cap above is
  exactly the kind of finding that review is for).
- Docker (`linux/amd64`) verification of every touched crate's test suite
  plus `wasm-conformance --test testsuite_conformance
  corpus_matches_the_committed_baseline` before pushing.
