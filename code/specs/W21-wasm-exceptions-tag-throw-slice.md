# W21 — Exceptions proposal, first slice: real conformance for `tag` + `throw`

## Purpose and how this slice was chosen

`code/specs/W20-wasm-gc-i31-conformance.md` picked up the GC-continuation
epic's smallest separable slice (`i31.wast`) and left a genuine
prioritization pass for the *next* slice as this session's job. This spec
is that pass, done for real against live state (the pinned testsuite tree,
the real exception-handling proposal text, and this repo's current code),
not a rubber stamp of a prior scan.

### Re-checking every candidate first

- **GC continuation beyond `i31`** (`call_ref.wast`, `br_on_null.wast`,
  `struct.wast`, `array.wast`, `ref_eq.wast`): re-fetched
  `call_ref.wast` at the pinned SHA
  (`28864811cf03bdbf880733786148feaba339582d`) directly. Its very FIRST
  module already needs `(ref $ii)` — a non-null CONCRETE function
  reference type as a parameter — plus `elem declare func`, a `global
  (ref $ll) (ref.func $fac)`, and `call_ref` itself. `wasm_types::ValueType`
  still has no non-null/nullable distinction for a concrete type index (only
  `Funcref`/`Externref`/`I31ref` as single opaque variants), exactly as W20
  found. Every other file in this family is entangled the same way (W20's
  own investigation, re-confirmed unchanged: nothing in this repo's type
  system moved since that PR merged). Not a viable next slice — still
  correctly XL/blocked, unchanged from W20.
- **`memory64`**: live-fetched the real corpus tree. `memory64.wast` (7930
  bytes) is real, but the pinned tree has TWENTY-FIVE separate `*64.wast`
  sibling files (`address64.wast`, `align64.wast`, `binary_leb128_64.wast`,
  `bulk64.wast`, `call_indirect64.wast`, `data0`/`data1` 64-bit variants,
  `endianness64.wast`, `float_memory64.wast`, `load64.wast`,
  `memory64-imports.wast`, `memory_copy64.wast`, `memory_fill64.wast`,
  `memory_grow64.wast`, `memory_init64.wast`, `memory_redundancy64.wast`,
  `store64.wast`, `table64.wast`, `table_copy64.wast`, `table_fill64.wast`,
  `table_get64.wast`, `table_grow64.wast`, `table_init64.wast`,
  `table_set64.wast`, `table_size64.wast`), each testing a 64-bit-indexed
  variant of an already-vendored 32-bit file. `wasm_types::MemoryType` has
  no index-width flag at all today, and memory64 changes the *dynamic
  operand type* (i32 address -> i64 address) of every single memory- and
  table-touching opcode (`i32.load`...`i64.store32`, `memory.size`/`.grow`/
  `.fill`/`.copy`/`.init`, every table op) — this is a systemic, cross-
  cutting change to nearly every memory-adjacent opcode handler in
  `wasm-execution`, not a narrow one. Confirmed XL, no small first bite
  smaller than "support a second memory-index width everywhere."
- **Real threading (`wait`/`notify`)**: re-read `wasm-execution/src/lib.rs`'s
  actual `call_function` (WASM10/W12) directly, current `origin/main`
  state. Confirmed unchanged from W20's own finding: `call_function` spawns
  exactly one dedicated OS thread via `Builder::spawn_scoped`, then the
  spawning thread's very next statement is `.join()` on the handle before
  doing anything else — a synchronous spawn-then-block, never two threads
  executing WASM at once. `memory.atomic.wait32`/`wait64`/`notify` need a
  second *actually concurrent* agent able to wake a blocked one; nothing
  about this design provides that. Still architecturally blocked, still
  not a viable slice — re-confirmed against the real current code, not
  assumed from the prior write-up.
- **Component model / JIT tier**: W07's own assessment (dedicated scoping
  investigation needed; blocked on a nonexistent `wasm-to-iir` lowering
  pass, respectively) — unchanged, not revisited here.

### The exceptions proposal: live-checked against the real spec text and the real corpus

Live-fetched `WebAssembly/exception-handling`'s current
`proposals/exception-handling/Exceptions.md` (the up-to-date, post-Oct-2023
`exnref`-based revision, not the older legacy proposal) for the real binary
encoding, and the real pinned-SHA testsuite tree for the real corpus file
sizes. Confirmed facts (not assumed from memory):

- New opcodes: `throw` (`0x08`, immediate: `tag_index: varuint32`),
  `throw_ref` (`0x0A`, no immediate), `try_table` (`0x1F`, immediates:
  `blocktype`, then `catch_count: varuint32`, then that many catch
  clauses). A catch clause is itself `clause_kind: u8` (`0x00 catch` /
  `0x01 catch_ref` / `0x02 catch_all` / `0x03 catch_all_ref`) plus
  `tag_idx`+`label_idx` (catch/catch_ref) or just `label_idx`
  (catch_all/catch_all_ref).
- New module section: a **tag section**, binary id `13`, positioned in
  the file between the memory section and the global section (WASM's
  established convention — like the MVP's own `datacount` section, id 12,
  sitting positionally between `elem` and `code` — a section's numeric id
  is not the same thing as its file position). A tag type is `{attribute:
  u8 (0 = exception), type: varuint32 (a func-type index; its `results`
  MUST be empty)}`. New import/export kind byte `4` = Tag.
- New value type `exnref` (byte `-0x17`), needed only by `catch_ref`/
  `catch_all_ref`/`throw_ref` (the "reify a caught exception so it can be
  rethrown" half of the proposal).
- Real corpus files at the pinned SHA: `tag.wast` (976 bytes), `throw.wast`
  (1920 bytes), `throw_ref.wast` (3234 bytes), `try_table.wast` (13554
  bytes).

Fetched and read all four files' full real content (not summarized from a
changelog):

- **`tag.wast`**: its first module is pure tag-section syntax — bare
  `(tag)`, `(tag (param i32))`, `(tag (export "t2") (param i32))`, a named
  `(export "t3" (tag 3))` — no `exnref`, no `throw`, no `try_table` at all.
  Its second module exercises tag *imports* (`(tag $t0 (import "test" "t2")
  (param i32))` and the explicit `(import "test" "t3" (tag $t1 (param i32
  f32)))` form) plus two `assert_invalid` cases for "non-empty tag result
  type". Its LATER "link-time typing" modules use `(rec (type $t1 (func))
  (type $t2 (func)))` — recursive type groups, which this repo's type
  system has never supported (same gap W20 already named for GC) — those
  modules, and only those, are expected to fail to build.
- **`throw.wast`**: every module lives in ONE `(module ...)` block (W14's
  per-module isolation is no help *within* a single module — confirmed by
  re-reading `code/specs/W14-wasm-conformance-lazy-module-build.md`, whose
  isolation unit is a whole `(module ...)` directive, not a single
  function). 11 of its 12 top-level directives (`assert_return`/
  `assert_exception`/`assert_invalid` on `throw-if`/`throw-param-*`/
  `throw-polymorphic*`, plus 3 `assert_invalid` type-mismatch/unknown-tag
  cases) need only `tag` + `throw` + a new `assert_exception` script
  directive. Exactly ONE test function (`test-throw-1-2`) additionally uses
  `try_table (catch $e-i32-i32 $h) (call $throw-1-2)` to actually CATCH and
  recover a thrown value — this is the one case this slice cannot make
  pass (see "Explicitly out of scope" below) — but because it shares the
  module with the other 11, `try_table`'s own TEXT SYNTAX still has to be
  at least parseable/buildable, or W14's whole-module isolation would zero
  out the other 11 real passes too. This is the key sizing fact that makes
  `throw.wast` bigger than "just `tag` + `throw`" in scope, but still far
  smaller than full exception handling.
- **`throw_ref.wast`**: every single one of its 6 exported test functions
  uses `try_table (catch_ref ...)`/`(catch_all_ref ...)` and `exnref`
  locals — fully entangled with the "reify and rethrow" half of the
  proposal this slice deliberately excludes. Not viable this slice.
- **`try_table.wast`** (13554 bytes, by far the largest of the four): a
  read of its content confirms heavy use of `catch_ref`/`catch_all_ref`/
  `exnref` and multi-level real catch-and-recover control flow throughout.
  Not viable this slice.

### What actually is separable: `tag.wast` + `throw.wast`, with `try_table` treated as an opaque (non-catching) block

The concrete scope decision, and why it's honest rather than a shortcut:
this slice implements `try_table`'s *text syntax, module-structure
validation, and control-flow shape* for real (parses, type-checks, opens a
real block-shaped control frame, closes on `end`) but deliberately does
**not** implement catch-clause matching — a thrown exception inside a
`try_table` body is never intercepted, so it always propagates out exactly
as if `try_table` were a plain `block`. This is not a shortcut disguised as
a feature: it is the literal, spec-correct behavior for the one case the
real spec itself defines as "no matching catch clause: implicitly
rethrown" — this slice just never has a matching catch clause, because it
never looks for one. The corpus's own `test-throw-1-2` needs *cross-function*
catching (the throw happens inside a **called function**, `$throw-1-2`,
and must be caught by the **caller's** `try_table`) — real catch-matching
even for same-function-only throws would not make this one case pass, so
building partial matching machinery would add real implementation risk for
zero additional real conformance credit against this specific corpus. That
tradeoff is worth re-examining the day a corpus file needing same-function
catching is vendored, not before.

## Scope

### In scope

1. **`wasm-types`**: `ExternalKind::Tag = 0x04` (matches the real spec's
   import/export kind byte exactly), `ImportTypeInfo::Tag(u32)` (the tag's
   function-type index), `WasmModule.tags: Vec<u32>` (module-defined tags'
   type indices, combined imported+defined index space assigned the same
   "imports first, then declaration order" convention every other index
   space in this repo already uses).
2. **`wasm-module-encoder`**: `ExternalKind`/`ImportTypeInfo` gained a
   variant each, so `encode_import`'s exhaustive match needs a `Tag` arm
   (compile-time requirement, not vendored-corpus-driven — this crate's
   own text-to-binary path isn't exercised by `wasm-wast-parser`'s corpus
   pipeline, matching W20's own precedent of not touching this crate,
   but the workspace must still compile per this repo's own "run `cargo
   build --workspace`" lesson).
3. **`wasm-validator`**: import/export tag-count bookkeeping (mirroring
   `imported_functions`/`imported_globals`), a "tag type index in bounds"
   check, a "tag's function-type `results` must be empty" check (real spec
   rule — the exact rule `tag.wast`'s own two `assert_invalid` cases named
   "non-empty tag result type" probe, though this harness's own
   `grade_assert_invalid` only checks *that* a module is rejected, never
   the message text — see `wasm-conformance`'s own doc comment), an export-
   index-bounds `ExternalKind::Tag` arm, and `type_check.rs` rules for
   `throw` (pop the tag's param types, `unknown tag` on an out-of-bounds
   index, mark the rest of the block unreachable — the same shape every
   other unconditional-exit instruction already uses) and `try_table`
   (decode blocktype exactly like `block`, decode+bounds-check the catch
   clause list, `push_ctrl` a plain `FrameKind::Block` — no new
   `FrameKind` needed, since this slice's `try_table` is control-flow-
   identical to `block` in every way the type-checker cares about).
4. **`wasm-execution`**: `TrapError` gains `is_exception: bool` (default
   `false` via the existing `TrapError::new` constructor; a new
   `TrapError::exception(msg)` constructor sets it `true`) so a thrown,
   uncaught WASM exception is distinguishable from an ordinary trap without
   changing `TrapError`'s public shape for any existing caller.
   `decode_function_body` gains explicit handling for `0x1F` (decode
   blocktype via the same logic `"blocktype"` immediates already use, then
   read-and-discard the catch-clause list so the following body
   instructions decode at the right position — mirrors the existing
   `0xD0`/`ref.null` precedent of a single-byte opcode with its own custom
   immediate shape outside the generic `immediates: &[...]` table) and
   `throw` is registered in `wasm_opcodes::OPCODES` with a new `"tagidx"`
   generic immediate kind (a plain LEB128 index, decoded exactly like
   `funcidx`/`localidx`/etc. already are). `build_control_flow_map` treats
   `0x1F` as an opener alongside `0x02..=0x04` (so its matching `0x0B` is
   found the same way). The `0x1F` execution handler is a near-verbatim
   copy of `0x02`'s (push a `Label`, not a loop, `target_pc` = the matched
   `end`) — genuinely nothing else needed, because propagating an
   uncaught error through a "block" is already exactly what happens today
   for every other trap: nothing intercepts it. `throw`'s execution handler
   is `Err(TrapError::exception(...))`, unconditionally — deliberately
   does not bother re-popping/validating the tag's argument values at
   runtime, since validation already guaranteed they were there and
   execution terminates immediately either way.
5. **`wasm-wast-parser`**: `tag` as a fifth inline-import-sugar kind
   (alongside `func`/`table`/`memory`/`global`), a `tag_names` index space,
   `(tag $name? (param ...)*)`/`(export "x" (tag $idx))`/inline `(export
   ...)` on a tag/explicit-and-inline `(import ...)` forms for tags, `throw
   $tag` instruction encoding (folded + flat), and `try_table` as a new
   structured-instruction form (alongside `block`/`loop`/`if`) in both the
   folded and flat/stream encoders — parses `(catch $tag $label)`/
   `(catch_all $label)` clauses (and, for completeness/robustness even
   though this slice's own corpus never uses them, `catch_ref`/
   `catch_all_ref`, since the immediate SHAPE costs nothing extra once the
   `catch`/`catch_all` shapes exist and leaving a silently-unparseable gap
   right next to a shape this crate DOES support is worse than the small
   marginal cost of finishing the enum) into the real binary catch-clause
   encoding, resolving tag names via `tag_names` and label names via the
   existing `resolve_label` machinery.
6. **`wasm-wast-parser`'s `script.rs`**: a new `Directive::AssertException
   { action: Action }` (the real corpus's own shape — always a bare
   `(assert_exception (invoke ...))`, never a message string or a module
   form, unlike `assert_trap`).
7. **`wasm-conformance`**: `ActionError::Exception(String)` (parallel to
   the existing `Trap(String)`, chosen by checking `TrapError.is_exception`
   at the one call site that converts a `TrapError` into an `ActionError`),
   `DirectiveKind::AssertException`, and grading: `Directive::AssertException`
   passes only on `ActionError::Exception`, fails on a normal return OR on
   a plain (non-exception) trap — real spec semantics: a trap and an
   uncaught exception are different outcomes, so `assert_exception` must
   not accept a trap.
8. **Vendor `tag.wast` and `throw.wast` verbatim** (pinned SHA
   `28864811cf03bdbf880733786148feaba339582d`), add both to
   `TESTSUITE_FILES`, regenerate the baseline.

### Explicitly out of scope (this slice)

- **`catch_ref`/`catch_all_ref`/`throw_ref`/`exnref`** — the "reify a
  caught exception, rethrow it" half of the proposal. `catch_ref`/
  `catch_all_ref`'s TEXT SYNTAX is parsed (see above — cheap once
  `catch`/`catch_all` exist) but produces no `exnref` value at runtime and
  is never reachable as MATCHING catch behavior, since no catch clause of
  any kind ever matches in this slice (see next point). `throw_ref` the
  INSTRUCTION is not implemented at all (no vendored file in this slice
  needs it).
- **Real catch-clause matching** (`try_table` actually intercepting a
  thrown exception and branching to a catch label with its arguments) —
  the core design decision this spec makes explicit above. A future slice
  that wants `throw_ref.wast`/`try_table.wast`/the one held-out
  `test-throw-1-2` case in `throw.wast` needs this, plus `exnref`, plus (per
  the corpus's own real content) cross-function propagation-then-catch,
  which is a materially bigger, cross-cutting change to how errors unwind
  through `call_function_inner`'s nested Rust call stack.
- **`(rec ...)` recursive type groups** — `tag.wast`'s own "link-time
  typing" modules need this; they are expected to (and, per W14, safely
  can) fail to build without affecting the file's other, real passes. Same
  gap W20 already named for GC, unchanged.
- **memory64, real threading, the component model, the JIT tier** —
  unchanged from W07/W20's own assessments, re-confirmed live above.

## Verification plan

- Unit tests: `tag`/`throw`/`try_table` text-syntax parsing (folded +
  flat, inline and explicit import/export sugar, `catch`/`catch_all`/
  `catch_ref`/`catch_all_ref` clause encoding) in `wasm-wast-parser`;
  `throw`'s pop-and-mark-unreachable rule, `try_table`'s block-shaped
  push/pop, the two `assert_invalid` shapes (unknown tag, tag-arity
  mismatch), and the "tag result type must be empty" module-level check in
  `wasm-validator`; `TrapError::exception`'s `is_exception` flag,
  `throw`'s runtime trap, and `try_table`'s pass-through (a body that
  throws is NOT caught; a body that returns normally is) in
  `wasm-execution`; `ActionError::Exception` grading (`assert_exception`
  passes on an exception, fails on a return, fails on a plain trap) in
  `wasm-conformance`.
- Vendor `tag.wast` + `throw.wast`, regenerate the baseline, and diff
  against the pre-change baseline: zero regressions on any already-
  vendored file. `tag.wast`'s first two modules (no `(rec ...)`) grade for
  real; its later "link-time typing" modules grade `NotYetSupported`.
  `throw.wast` grades 11/12 directives as real `Pass` (10
  `assert_return`/`assert_exception` + 3 `assert_invalid`, one of which —
  `test-throw-1-2` — the ONE case out of scope) with exactly one deliberate,
  reviewed `Fail` for `test-throw-1-2` (documented here, not a surprise
  found later).
- `/security-review` before push, per this repo's standing workflow.
- Docker (`linux/amd64`) verification of `cargo test -p wasm-execution
  --lib` and `cargo test -p wasm-conformance --test testsuite_conformance
  corpus_matches_the_committed_baseline` before pushing, per this
  campaign's standing discipline.
