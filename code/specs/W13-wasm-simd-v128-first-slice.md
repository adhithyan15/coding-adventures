# W13 — WASM SIMD (v128): real encoding, value-representation decision, and a genuinely bounded first slice

## Purpose

SIMD is the next epic on `code/specs/W07-wasm-post-mvp-epics.md`'s own suggested
sequencing, after tail calls (WASM16), funcref/externref (WASM17), and plain
atomics (WASM18) — all shipped. That doc already flagged SIMD as **XL** and
explicitly warned its own `0xFD`-prefix claim needed verifying against the real
spec before implementing, rather than trusted from a comment. This spec does
that verification, makes the one load-bearing design decision (how a 128-bit
lane vector is represented on this engine's existing typed stack), and scopes
a real, bounded PR-1 — while being honest that even the *narrowest possible*
slice is **L-sized**, not the S/M this repo would prefer to start with.

## What's confirmed, not assumed

### The real encoding

Verified against the upstream spec (`webassembly.github.io/spec/core/binary/
instructions.html`): every SIMD instruction has **a one-byte prefix (`0xFD`,
confirming the repo doc's guess) followed by the actual opcode encoded as a
variable-length (LEB128) unsigned integer** — e.g. `0xFD 0x00` is `v128.load`,
`0xFD 0x0C` is `v128.const`.

**This is a real, structural gap relative to every prefixed-opcode family this
crate has shipped so far.** `wasm-execution::decode_function_body` decodes the
existing two-byte-prefix families — `0xFB` (WasmGC), `0xFC` (trunc_sat/bulk-
memory), `0xFE` (atomics) — as a **bare single byte** read immediately after
the prefix (works today only because every sub-opcode value in those three
families happens to be `<128`). SIMD's sub-opcode space runs well past 127
(relaxed-SIMD lands in the `0x100`+ range), so a correct SIMD decoder needs a
genuine LEB128 read at the sub-opcode position — the existing single-byte
pattern cannot be copy-pasted. This is new decoder infrastructure, not a new
match arm on an existing one.

### The value-representation decision

`virtual-machine::Value` (the shared, language-agnostic typed-stack value used
by every VM-backed frontend in this repo, not just WASM) has variants up to
`Str(String)` (24 bytes) and `Code(Box<CodeObject>)` — so `Value`'s existing
size envelope already accommodates a 16-byte payload with room to spare; a
naive `Value::V128([u8; 16])` addition would **not** measurably grow the enum
(the size argument some might expect against this approach doesn't actually
hold, confirmed by inspection, not assumed).

**The real reason to reject a `Value::V128` variant anyway**: this repo
already has a working, precedented pattern for exactly this shape of problem —
WasmGC's struct/i31 heap. `WasmExecutionContext::gc_heap: Vec<Option<GcStruct>>`
holds real heap-allocated data **local to the WASM execution context**, never
touching the shared `virtual-machine::Value` enum at all; `WasmValue::Ref(
Option<u32>)` rides the *existing* `Value::Int` slot as a handle into that
heap, tagged on the typed stack with WASM's own real type byte (`0x6E`,
anyref) via the `value_type: u8` field `TypedVMValue` already carries.

**Decision: v128 follows the identical shape.** A new `WasmExecutionContext::
v128_heap: Vec<[u8; 16]>` field; a new `WasmValue::V128(u32)` variant carrying
a handle into it; that handle rides the existing `Value::Int` slot on the
typed stack, tagged with WASM's real `v128` type byte (`0x7B`) the same way
`Ref` already uses `0x6E`. This keeps `virtual-machine`'s shared `Value` enum
**completely untouched** — zero blast radius into the other language
frontends built on this shared VM crate — and reuses a pattern this codebase
has already proven correct rather than inventing a new one. (Unlike GC
objects, v128 values need no mark-sweep collection: they're plain `Copy`
16-byte arrays, immutable once created, so `v128_heap` can simply grow for the
duration of one `call_function` invocation the same way `gc_heap` already does
before GC was added to it — no premature optimization needed for a first
slice.)

### Blast radius, honestly counted

Grepped, not estimated: `ValueType::` match sites appear 157× in
`wasm-execution/src/lib.rs`, 80× in `wasm-types/src/lib.rs`, 70× in
`wasm-opcodes/src/lib.rs`, 50× in `wasm-validator/src/type_check.rs`, 22× in
`wasm-wast-parser/src/module.rs`, plus real hits in `iir-to-wasm` and
`wasm-module-parser`. Not every hit is an exhaustive match needing a new arm,
but the two confirmed-exhaustive ones — `ValueType::byte_tag`/`ValueType::
encode` in `wasm-types/src/lib.rs` (~189-235) — each need a `V128` arm on
day one, and `WasmValue::to_typed`/`from_typed` (`wasm-execution/src/lib.rs`
~128-193, the exhaustive round-trip through `TypedVMValue`) is the site that
structurally could not accept a V128 payload before this spec's handle
decision existed.

### Corpus status

`wasm-conformance/tests/fixtures/testsuite/` has zero `simd_*.wast` files
today. The `NOTICE` file (pinned commit `28864811cf03bdbf880733786148feaba33
9582d`) already documents SIMD as one of the categories excluded by policy so
far. `fetch_testsuite.py`'s `PROPOSAL_FILES` dict already has the exact
mechanism this needs — atomics' entries look like `"atomic.wast":
"proposals/threads/atomic.wast"` — SIMD files live under `proposals/simd/
*.wast` at the **same pinned SHA**, so vendoring is additive dict entries plus
a script re-run, no new pin needed. `wasm-conformance`'s own `assert_return`
grading (bit-exact comparison) does not yet know how to compare two 16-byte
values — this needs a real `WasmValue::V128` arm in its comparison logic
before even one SIMD `assert_return` case can grade as anything but
`NotYetSupported`.

## Honest sizing: even the minimum slice is L

The narrowest defensible first slice this spec considered — `v128.const` +
`i32x4.splat` + `i32x4.add` + one `i32x4` comparison, ONE lane width, ONE
vendored conformance file passing end-to-end — still requires, simultaneously
and non-optionally:

1. The `v128_heap`/`WasmValue::V128(u32)` representation above, threaded
   through `wasm-types` and `wasm-execution`.
2. A genuinely new LEB128-based sub-opcode decode shape for the `0xFD` prefix
   in `decode_function_body` (cannot reuse the existing single-byte
   `0xFB`/`0xFC`/`0xFE` pattern).
3. `wasm-opcodes` table entries for the handful of opcodes in scope, using the
   new LEB128-keyed lookup shape rather than the flat byte table every other
   family uses.
4. `wasm-wast-parser` support for `v128.const`'s own lane-literal text syntax
   (a 16-byte list, or a per-lane-width shorthand like `i32x4 1 2 3 4`) — this
   repo's parser has no existing analogous literal grammar to extend.
5. `wasm-validator` type rules for the in-scope opcodes (mechanically similar
   to existing families once the `V128` `ValueType` arm exists, but still new
   code).
6. `wasm-conformance` gaining a real `V128` case in its bit-exact comparison
   logic, plus fresh vendoring of at least one `simd_*.wast` file.

None of these six are optional even for the narrowest possible slice — unlike
prior epics (WASM16/17/18), where the type-system/decoder/parser plumbing
already existed and only new opcode handlers were novel, SIMD's very first
opcode requires standing up genuinely new shared infrastructure across five
crates at once. **This makes even PR-1 an L by this repo's own rubric** ("two
independently-substantial pieces": the representation/decoder infrastructure,
and the opcode/validator/parser plumbing for the handful of in-scope ops) —
not the S/M this repo has preferred to start epics with. This spec reports
that honestly rather than under-scoping PR-1 to look smaller than it is.

## Recommended staged shape

Matching `W07`'s own recommendation and this session's established spec-then-
implementation-slices pattern, but naming the stages explicitly given the
confirmed L size:

1. **This spec** (sign-off only, no code) — the value-representation and
   encoding decisions, reviewed and merged before any implementation, since
   getting either wrong invalidates every later PR's landed code.
2. **PR 1 — infrastructure + smallest real vertical slice**: `v128_heap` +
   `WasmValue::V128` + the LEB128 `0xFD`-prefix decoder shape + `ValueType::
   V128` across its exhaustive matches + `v128.const`/`i32x4.splat`/
   `i32x4.add` (three opcodes, one lane width) + matching validator rules +
   `v128.const`'s text-literal syntax in `wasm-wast-parser` + `wasm-
   conformance`'s `V128` comparison arm + vendor `simd_const.wast` (or the
   smallest real upstream file that only needs these three ops) and get it to
   a real, graded pass. This is genuinely the whole "one-time infrastructure
   tax" — expect it to feel disproportionately large relative to the 3 opcodes
   it delivers, because it is.
3. **PR 2+ — opcode family slices**, now cheap relative to PR 1: arithmetic
   across the remaining lane widths (`i8x16`/`i16x8`/`i64x2`/`f32x4`/`f64x2`),
   then extract/replace-lane, then comparisons, then shuffle (its own text-
   syntax wrinkle — 16 inline lane-index immediates), then loads/stores, with
   relaxed-SIMD and saturating-arithmetic edge cases picked up as their own
   later slices. Each should land independently, vendoring more of the
   `proposals/simd/*.wast` corpus as coverage grows.

## Non-goals (this spec)

- Implementing any opcode — this is a design-and-scoping spec only.
- Relaxed SIMD (deliberately implementation-defined rounding/ordering,
  explicitly out of scope until the core proposal is solid).
- Full 230+ opcode coverage in one PR, or even one session.
- Changing `virtual-machine::Value` — the whole point of the handle design is
  that this shared crate stays untouched.
- SIMD support in `iir-to-wasm` or any other WASM-emitting frontend — this
  spec covers `wasm-execution`'s interpreter side only.

## Verification (once implementation starts)

- `cargo test -p wasm-wast-parser -p wasm-execution -p wasm-validator -p
  wasm-conformance` green after PR 1, including a real, non-`NotYetSupported`
  pass on at least one vendored `simd_*.wast` case.
- `wasm-conformance` baseline regen showing the new file(s) at real pass
  counts, zero regressions elsewhere (same full-baseline-diff discipline
  every prior WASM epic in this session used).
- Confirm `virtual-machine`'s own existing test suite is unchanged — the
  whole point of the handle-based design is that this shared crate needs no
  changes at all; its test suite passing unmodified is the check that holds.
