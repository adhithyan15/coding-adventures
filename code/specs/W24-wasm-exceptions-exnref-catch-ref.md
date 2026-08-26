# W24 — Exceptions proposal, fourth slice: real `exnref`, `catch_ref`/`catch_all_ref`, `throw_ref`

## Purpose and how this slice was chosen

This session's task was to genuinely re-assess whether building minimal
non-null-CONCRETE-reference-type support (`(ref $t)` for a concrete `$t`,
as opposed to the nullable `funcref`/`externref`/`anyref`/`exnref` this
repo already has) is the highest-leverage next move for either of the two
epics it blocks — GC continuation's `call_ref` (`code/specs/
W20-wasm-gc-i31-conformance.md`) or the exceptions proposal's `catch_ref`/
`exnref` (`code/specs/W23-wasm-exceptions-cross-instance-tag-identity.md`,
which re-confirmed W20's own finding) — or whether it's genuinely larger
and a different, smaller win should be picked instead.

### Re-deriving the non-null-concrete-ref-type question directly (not trusting W20/W23's own prior conclusion)

Live-fetched the real corpus files at the pinned SHA
(`28864811cf03bdbf880733786148feaba339582d`) rather than reasoning from
either prior spec doc's own summary:

- **`call_ref.wast`**: its VERY FIRST module already requires
  `(ref $ii)` as a function PARAMETER type (`(func $apply (param $f (ref
  $ii)) ...)`), `(ref $ll)`/`(ref $lll)` as GLOBAL types, `elem declare
  func`, and `ref.func` producing a non-null concrete function reference.
  There is no way to get ANY real, gradable corpus value from this file
  without non-null concrete function-type references — confirmed by
  reading the file directly, not inferring from a summary. Beyond the
  type-system gap itself, `call_ref` also needs an actual RUNTIME
  representation of "a reference to a specific function" as a first-class
  value (produced by `ref.func`, storable in locals/globals/params) — a
  new representation decision with the same shape of blast radius W13's
  `V128` addition had (a value that needs plumbing through every
  exhaustive match on `WasmValue`), PLUS the `call_ref` instruction itself
  (a new indirect-call mechanism distinct from `call_indirect`'s
  table-based one). This is genuinely not separable into a smaller
  sub-slice — confirmed independently a third time (W20, W23, and now
  this session), no smaller entry point exists.
- **`throw_ref.wast`**: by contrast, EVERY directive in this file (5
  `assert_return`, 2 `assert_invalid`, 7 `assert_exception`) uses only
  plain, abstract `exnref` — never a concrete `(ref $t)` anywhere.
  Confirmed by reading the entire file: no `(ref $t)`, no `(rec ...)`, no
  heap-type hierarchy at all.
- **`try_table.wast`'s `catch_ref`/`catch_all_ref` directives**: mostly
  the same story — the `throw-catch_ref-param-{i32,f32,i64,f64}` cluster
  (10 `assert_return` directives) and several `assert_invalid` cases use
  only plain `exnref`. Exactly ONE module in the file (the
  `catch`/`catch_ref1`/`catch_ref2`/`catch_all_ref1`/`catch_all_ref2`
  cluster near the file's own `(ref $t)`/`(ref exn)`/`(ref null exn)`
  distinctions) genuinely needs non-null concrete AND non-null abstract
  reference types together — confirmed unreachable without the same gap
  `call_ref` needs, and left out of scope (see below).

**This is the real finding, and it's more precise than either W20 or
W23's own framing**: non-null-concrete-reference-type support is not a
single monolithic gap that either blocks everything or nothing. `exnref`
itself — a real, reified "handle to a caught exception" value — is
SEPARABLE from the `(ref $t)` gap entirely, because `exnref` is a plain
NULLABLE, ABSTRACT reference type (no concrete type parameter, no
non-null/nullable distinction this repo needs to model — the exact same
simplification this repo already makes for `funcref`/`externref`/
`anyref`, per `wasm_types::ValueType`'s own doc comments). Building a real
`exnref` needs a runtime representation, yes, but `wasm-execution`
already has the EXACT shape needed: `WasmValue::Ref(Option<u32>)`, a
handle into a per-call heap (`gc_heap`/`v128_heap`'s own established
pattern) — no new `WasmValue` variant required, unlike `call_ref`'s
function-reference need or `V128`'s original 128-bit-payload need. Only
the LAST module of `try_table.wast` (the one mixing `(ref $t)`/`(ref
exn)`) is blocked on the harder gap.

### Why this beats forcing a "minimal non-null-ref-type" slice for `call_ref`

A minimal `(ref $t)` slice sized ONLY for `call_ref.wast`'s first module
would still need: a new `ValueType` representation (nullable + concrete
type-index pair, or a non-null/nullable subtype relationship — genuinely
new type-system surface, not additive), a new `WasmValue` runtime
representation for function references, AND the `call_ref` instruction's
own indirect-call machinery. That's the same order of magnitude as this
session's OWN estimate of the full slice — there is no genuinely smaller
sub-slice of `call_ref` than "all of it." `exnref`/`catch_ref`/
`throw_ref`, deliberately avoiding `(ref $t)`, is the real smaller win
this round.

### Re-checking memory64 and the other W07 candidates (not re-litigating in depth — W23 already did this live this same session cycle)

`memory64` remains unscoped by ANY corpus coverage in the pinned
`WebAssembly/testsuite` tree (confirmed unchanged — it lives in a
separate, not-yet-merged proposal repo at this SHA), so there is no real
conformance win available there regardless of implementation effort. The
component model, real threading, and the JIT tier remain architecturally
blocked/out of scope per W07's own unchanged assessment. `exnref`/
`catch_ref`/`throw_ref` is the clear pick: real, already-vendorable corpus
value, no new value-representation blast radius, no new type-system gap.

## Scope

### In scope

1. **A real, reified `exnref` value**, reusing `WasmValue::Ref(Option<u32>)`
   (no new `WasmValue` variant): `WasmExecutionContext` gains
   `exception_heap: Vec<ExceptionPayload>`, the same "handle into a
   per-top-level-call heap" shape `gc_heap`/`v128_heap` already
   establish. A `catch_ref`/`catch_all_ref` clause that matches pushes a
   new entry (`push_caught_exception`, bounded by new
   `MAX_EXCEPTION_HEAP_LEN` = 1,000,000, mirroring `push_v128`/
   `MAX_V128_HEAP_LEN`'s security-review shape) and hands back
   `WasmValue::Ref(Some(handle))`.
2. **Real `catch_ref`/`catch_all_ref` matching** (`try_catch_exception`):
   these clause kinds now match under the EXACT same rule as their
   non-`_ref` counterparts (`catch_clause_tag_matches`/unconditional
   respectively) instead of never matching (W21/W22/W23's own deliberate,
   narrower scope). On a match: `CatchRef` pushes the tag's argument
   values THEN the reified `exnref` (per the real spec's own
   result-type order, confirmed against `try_table.wast`'s
   `(result i32 exnref)`-shaped block signatures); `CatchAllRef` pushes
   only the reified `exnref`.
3. **`throw_ref` (`0x0A`, no immediate)**: registered in `wasm-opcodes`
   (previously deliberately unregistered per W21's own scope note).
   `wasm-execution`'s handler pops a `WasmValue::Ref`, traps `"null
   exception reference"` if null, otherwise looks up the full
   `ExceptionPayload` in `ctx.exception_heap` and re-raises it verbatim
   (tag, tag identity, argument values all intact) via
   `TrapError::exception_with_payload` — a genuine re-throw, provably
   round-tripping the original payload (see the `wasm-execution` unit
   test that reifies a thrown `i32`-carrying exception via
   `catch_all_ref`, re-throws it with `throw_ref`, and confirms an OUTER
   plain `catch` recovers the original value).
4. **`wasm-validator`**:
   - `throw_ref` (`0x0A`) type rule: pop one `Exnref`, mark the rest of
     the block unreachable (same shape `throw`/`unreachable`/`br`/
     `return` already use).
   - `catch_ref`/`catch_all_ref` clauses now get a REAL arity/type check:
     the target label's declared type must equal exactly the tag's
     params (`catch_ref`) or nothing (`catch_all_ref`) followed by
     `Exnref`. Plain `catch`/`catch_all` are deliberately LEFT UNCHANGED
     (still no arity check — W21/W22's own scope boundary) to avoid any
     regression risk to already-passing `catch`/`catch_all` directives;
     only the `_ref` variants, which now have real runtime consequences,
     get the new check.
5. **Fixed a real, pre-existing blocktype-decoding gap surfaced while
   vendoring `throw_ref.wast`**: `exnref`'s single-value shorthand
   blocktype form (`(block $h (result exnref) ...)`, a REAL shape both
   `throw_ref.wast` and `try_table.wast` use) was never recognized by
   `decode_blocktype` (`wasm-validator`) or the matching `"blocktype"`
   operand decoder / `block_arity` (`wasm-execution`) — the same gap
   `0x7B`/`0x70`/`0x6F` (v128/funcref/externref) already hit and were
   fixed for (WASM17/SIMD), just never extended to `exnref` since nothing
   exercised it before this slice. Fixed by adding an explicit arm to all
   three sites, ordered BEFORE the generic signed-LEB128 type-index
   fallback.
6. **Security review finding, fixed at the source**: the byte initially
   chosen for that new arm (`0xE9`, matching `wasm-types`' own
   PRE-EXISTING — and, it turns out, incorrect — `ValueType::Exnref` wire
   encoding since W22) has its LEB128 continuation bit SET (`0xE9 >=
   0x80`), making it indistinguishable, in a blocktype context, from the
   leading byte of a genuine multi-byte type index: a module declaring
   234+ types could silently misparse a real type-index blocktype as
   `exnref`, corrupting the rest of that function body's decode. Root
   cause: `0xE9` is `-0x17`'s two's-complement-mod-256 byte
   (`-23 + 256`), not its correct single-byte SLEB128 encoding
   (`-23 & 0x7F = 0x69`) — every OTHER abstract reference type here
   happens to have its raw spec byte ALSO be its correct SLEB128 encoding
   (`funcref` `0x70`, `externref` `0x6F`, `anyref` `0x6E`, `i31ref`
   `0x6C`), which is what let this go unnoticed for two prior slices:
   `exn`'s value (`-0x17`) is the first one where the two representations
   diverge. Fixed by correcting `wasm-types::ValueType::Exnref::byte_tag`/
   `encode` to `0x69` (continuation bit clear — spec-correct, and safe:
   it can only ever be a complete standalone value, never a type-index
   prefix) and updating all three blocktype-decode sites to match. No
   existing test anywhere in the repo hard-coded the old `0xE9` value
   (confirmed via a repo-wide grep before changing it, distinguishing it
   from the unrelated `0xE9` SIMD sub-opcode `f32x4.max`, a completely
   different byte namespace), so this was a clean, non-breaking fix —
   confirmed via an unchanged conformance baseline (byte-identical JSON)
   after the correction.
6. **Vendor `throw_ref.wast` verbatim** (pinned SHA
   `28864811cf03bdbf880733786148feaba339582d`) into `wasm-conformance`,
   regenerate the baseline.

### Explicitly out of scope (this slice)

- **Non-null concrete reference types (`(ref $t)`)** — unchanged from
  W20/W23; confirmed a third time this session to be a genuinely
  larger, non-separable gap (see "Re-deriving" above). Blocks `call_ref`
  entirely and blocks exactly one module of `try_table.wast` (the
  `(ref $t)`/`(ref exn)` cluster) — that module's 5 `assert_return` + 2
  `assert_invalid` directives correctly stay `NotYetSupported`.
- **`(rec ...)` recursive type declarations, the `eq`/`any`/`none`
  abstract heap-type hierarchy, arrays** — unchanged from W20.
- **memory64, real threading, the component model, the JIT tier** —
  unchanged from W07/W23's own re-scoping.

## Verification

- **Unit tests** (`wasm-execution`): `catch_all_ref` reifies a real,
  non-null `exnref` handle (not a structural no-op); `throw_ref`
  re-raises a reified exception with its ORIGINAL tag/payload intact,
  recoverable by an OUTER plain `catch` (the strongest correctness
  signal — proves the heap entry carries the full exception, not just an
  opaque empty token); `throw_ref` on a null `exnref` traps.
- **Unit tests** (`wasm-wast-parser`): `throw_ref` encodes as the bare
  `0x0A` byte (no immediate) in both folded and flat text forms.
- **Unit tests** (`wasm-validator`): `throw_ref` pops `Exnref` and marks
  dead code; the file's own real `assert_invalid` shapes (`(func
  (throw_ref))`, `(func (block (throw_ref)))`, a `catch_ref`/
  `catch_all_ref` target label missing the `exnref`, a `catch_ref` target
  label whose leading types don't match the tag's own params) are all
  rejected; a matching `catch_ref` target label type is accepted. A
  pre-existing hand-built test (`valid_try_table_catch_all_and_ref_variants_parse_and_validate`)
  was updated to give `catch_ref`/`catch_all_ref` their own correctly-typed
  target labels, since it previously relied on the (now-closed) leniency
  this slice removes.
- **Real corpus, measured** (`cargo run --bin wasm_conformance_report`,
  full JSON diff against the pre-change baseline, confirming ONLY
  `throw_ref.wast` (new) and `try_table.wast` moved):
  - `throw_ref.wast` (new): `module` 1/1, `assert_return` 5/5,
    `assert_invalid` 2/2, `assert_exception` 7/7 — 100% real pass.
  - `try_table.wast`: `assert_return` 28p/10f/5nys → **38p/0f/5nys**;
    `assert_invalid` 5p/4nys → **7p/2nys**. The remaining
    not-yet-supported directives are exactly the one `(ref $t)`-dependent
    module, confirmed unrelated to this slice.
  - Aggregate: `assert_return` +15 pass, -10 fail; `assert_invalid` +4
    pass, -2 not-yet-supported; `assert_exception` +7 pass. Zero
    regressions anywhere else in the 140-file corpus.
- **Downstream consumers**: `cargo test -p lang-aot` and every other
  crate depending on `wasm-types`/`wasm-execution`/`wasm-validator`/
  `wasm-opcodes`/`wasm-wast-parser` (`wasm-conformance`, `wasm-runtime`,
  `twig-to-wasm`, `nib-wasm-compiler`, `brainfuck-wasm-compiler`,
  `ir-to-wasm-compiler`, `ir-to-wasm-validator`, `iir-to-wasm`) built/
  tested to confirm the `ValueType`/opcode-table changes don't break any
  of them.
- `/security-review` before push, per this repo's standing workflow.
- Docker (`linux/amd64`) verification of every touched crate's test suite
  plus `wasm-conformance --test testsuite_conformance
  corpus_matches_the_committed_baseline` before pushing.
