# W07 — Post-MVP WASM Epics: SIMD, Threads, Tail Calls, GC, Component Model, and a JIT Tier

## Purpose

This is a **planning document, not an implementation spec**. WASM04 through
WASM08 (this session) took the WASM stack from "structural validation only,
no multi-value, ~40 vendored conformance files" to a real instruction-level
type checker, multi-value block signatures, and a 47-file vendored corpus
passing at effectively 100% wherever a case is actually gradable
(`assert_return` 13874/13879, `assert_invalid` 909/962, `module` 108/108 —
see `wasm-conformance`'s own `CHANGELOG.md` for the exact, currently-true
numbers; this document does not attempt to keep them fresh, since they
drift with every future PR).

The WASM 1.0 MVP core is now genuinely solid. Everything past that —
SIMD, threads/atomics, tail calls, garbage collection beyond this repo's
existing narrow struct/i31 slice, the component model, and a JIT tier —
is a **separate proposal**, each with its own opcode encoding, its own
validation rules, and (for the four instruction-set proposals) its own
slice of the vendored testsuite currently sitting untouched in
`WebAssembly/testsuite` at the pinned commit. This document surveys each
one honestly — what it actually is, what this repo would need to build,
how big that really is — so a future session can pick one up with a real
implementation plan instead of starting from zero context. Each epic gets
its own dedicated `W0N-*.md` spec (`W08`, `W09`, ...) when someone
actually starts it; this document is the map, not the territory.

## How to read the sizing

Each epic below is sized as **S / M / L / XL**, calibrated against this
session's own completed work as a yardstick:

- **S** — comparable to WASM03 (sign-extension + trunc_sat: ~13 opcodes,
  one crate, one PR, half a session).
- **M** — comparable to WASM06 (a real algorithm, one new module, touches
  2-3 crates, one focused session with a security-review round).
- **L** — comparable to this session's WASM04+WASM06 combined: a new
  binary-encoding wrinkle in the parser AND a new algorithm in the
  interpreter/validator, each independently substantial, likely wants its
  own spec-first PR before implementation (matching WASM04's own
  spec-then-impl two-PR shape).
- **XL** — a genuinely new subsystem (a second value-representation
  scheme, a new memory model, a new module-linking story) that doesn't
  fit the existing "decode a byte stream into typed operations" shape at
  all. Expect multiple L-sized PRs.

---

## Epic 1: SIMD (fixed-width 128-bit vectors)

**Size: XL.** By far the largest single proposal in raw opcode count.

### What it is

Adds a fifth value type, `v128` (128 bits, no fixed lane interpretation —
lane width is per-instruction), and roughly 230+ new opcodes (the real
WASM spec's own count; `wasm-opcodes`' current doc comment only notes
that SIMD is out of scope for its table and uses a separate two-byte
prefix, without itself enumerating the count or confirming which prefix
byte -- verify the exact prefix (0xFD in the real spec, distinct from
the 0xFC prefix this repo's `trunc_sat`/bulk-memory sub-opcodes already
use) against the spec directly before implementing, not against that
comment) covering: lane-wise arithmetic across
`i8x16`/`i16x8`/`i32x4`/`i64x2`/`f32x4`/`f64x2` interpretations, splat/
extract/replace-lane, shuffles, saturating arithmetic, bitmask
extraction, and the "relaxed SIMD" sub-proposal (deliberately
implementation-defined rounding/ordering for a handful of ops, meant for
platforms where bit-exact determinism isn't required).

### What this repo would need

- **`wasm-opcodes`**: a new `0xFD`-prefixed two-byte decode table,
  mirroring the existing `0xFC` (trunc_sat) and `0xFB` (GC) prefix
  special-casing already in `wasm-execution`'s `decode_function_body` and
  now also in `wasm-validator`'s `type_check.rs` — but ~30x the opcode
  count of either of those.
- **`wasm-types`**: add `ValueType::V128`. Every place that currently
  matches exhaustively on the 4 numeric types (there are several, e.g.
  `wasm-validator`'s `type_check_numeric`, `wasm-execution`'s numeric
  register functions) needs a 5th arm.
- **`virtual-machine`**: the shared `GenericVM` typed stack has one slot
  per WASM value width today (`Value::I32`/`I64`/`Float(f64)` doing
  double duty for f32/f64 — see WASM13's fix). A 128-bit lane vector
  needs its own representation; this is the first WASM value type that
  doesn't fit in a `u64`/`f64`, so it's a genuine new case for the shared
  VM crate, not just "add an enum variant."
- **`wasm-execution`**: ~236 new opcode handlers. Highly mechanical once
  the lane-arithmetic helpers exist (most SIMD ops are "do the scalar op
  N times across lanes"), but there are real correctness traps: shuffle
  lane-index validation, saturating lane arithmetic reusing the same
  Rust-`as`-cast tricks WASM03 used for scalar trunc_sat, and NaN
  canonicalization per-lane (WASM13's f32 NaN-payload bug, but ×4 lanes,
  ×2 widths).
- **`wasm-wast-parser`**: `v128.const` literals have their own
  16-byte-list-or-lane-shorthand text syntax; SIMD instruction names are
  the least MVP-regular of any family (e.g. `i8x16.shuffle` takes 16
  inline lane-index immediates in the text form).
- **Corpus**: ~50 `simd_*.wast` files sitting in the pinned vendored
  commit already (confirmed via a live GitHub API tree listing — see
  WASM08's own fetch verification approach), completely untouched by
  today's `TESTSUITE_FILES` list.

### Recommended shape if picked up

Split by the same spec-first convention as WASM04: one spec PR designing
the `v128` representation choice up front (this is the one decision that,
if wrong, ripples through every other file — get it reviewed before
writing 236 opcode handlers), then implementation PRs in digestible
slices (e.g. "arithmetic + splat/extract/replace" as PR 1, "shuffle +
lane ops" as PR 2, "loads/stores + relaxed SIMD" as PR 3), each landing
independently rather than one enormous diff.

---

## Epic 2: Threads and Atomics

**Size: L.**

### What it is

Shared-memory multi-agent execution: a `shared` flag on memory limits,
atomic load/store/read-modify-write/compare-exchange opcodes (0xFE
prefix), and `memory.atomic.wait32`/`wait64`/`notify` for blocking
synchronization.

### What this repo would need

- A **new prefix decode table** (0xFE), same shape as SIMD's but far
  smaller (~50 opcodes).
- **Real thread support**, which this repo does not have anywhere in the
  WASM stack today — `wasm-execution`'s `GenericVM` is a single-threaded
  tree-walking interpreter. `memory.atomic.wait`/`notify` are
  meaningless without an actual second thread of execution able to
  `notify` a blocked one. This is the epic's real cost: it's not "add
  opcodes to the existing interpreter," it's "decide whether to actually
  support concurrent WASM execution at all," which is an architectural
  question this repo hasn't needed to answer yet (WASM10, already in the
  backlog and explicitly deprioritized as blocked, is a *much* smaller
  version of this same question — "run WASM on a dedicated thread with a
  guaranteed stack" — and even that was set aside as architecturally
  blocked on non-`Send` raw pointers in the current design).
- The plain (non-`wait`/`notify`) atomic ops *are* implementable without
  real threading (they can just be regular loads/stores/RMW against the
  single linear memory, since with one thread every atomic op is
  trivially atomic) — a legitimate **partial** slice: "atomic memory ops,
  no real concurrency" gets real conformance value without the
  architectural threading question. `wait`/`notify` would correctly stay
  `NotYetSupported`.

### Recommended shape if picked up

Do the partial slice (plain atomics against single-threaded memory)
first — it's roughly SIMD-epic-shaped but 5x smaller, and doesn't block
on the threading question at all. Treat "does this repo ever run WASM
concurrently" as its own separate architectural decision, informed by
(and probably sequenced after) WASM10's own resolution.

---

## Epic 3: Tail Calls

**Size: S.**

### What it is

Two new opcodes: `return_call` (0x12) and `return_call_indirect` (0x13).
Semantically: pop the current frame *before* making the call, so a
tail-recursive function runs in O(1) stack space instead of O(n).

### What this repo would need

- **`wasm-opcodes`**: 2 new entries.
- **`wasm-execution`**: this is where the real work is, and it's
  genuinely small in opcode count but touches the call machinery's
  actual frame-management logic (`call_function_inner`, per this
  session's own reading of that code) — the new opcodes need to *replace*
  the current frame rather than push a new one, which is a different code
  path than `call`/`call_indirect`'s existing "push a new frame" shape.
  WASM01's `MAX_CALL_DEPTH` guard exists specifically to catch unbounded
  *non-tail* recursion; tail calls are the intentional exception to that
  guard (a real tail call must never hit it, by design).
- **`wasm-validator`**: 2 new type rules, structurally identical to
  `call`/`call_indirect`'s existing ones (same param/result popping) —
  this part really is S-sized.
- **Corpus**: `return_call.wast`, `return_call_indirect.wast`,
  `return_call_ref.wast` (the last needs function-references, out of
  scope until GC's function-reference slice grows).

### Recommended shape if picked up

Good "next small epic" candidate after this session's run — single PR,
clear scope, the one real design question (how `call_function_inner`
replaces vs. pushes a frame) is well-contained and doesn't ripple
elsewhere.

---

## Epic 4: GC / Reference Types Beyond the Current Slice

**Size: L**, but note this repo already has a real foothold here.

### What already exists

`wasm-execution` already implements a working slice: `struct.new`,
`struct.get`, `struct.set`, `i31.new`, `i31.get_s`, `ref.test`,
`ref.null` (the `0xFB`-prefixed sub-opcodes `wasm-validator`'s
`type_check.rs` module doc comment already enumerates and explicitly
scopes around — see its own "out of W02 Phase 2's scope" note). This
backs a real Lisp cons/car implementation (`wasm-runtime`'s own
`build_cons_car_wasm` test) and a working GC heap
(`code/specs/W04-wasm-gc.md`).

### What's missing

The full reference-types + GC proposal adds: `funcref`/`externref` as
first-class value types usable in locals/globals/tables (not just the
struct-heap slice this repo has), `array` types (fixed and growable),
`ref.func`, `br_on_null`/`br_on_non_null`/`br_on_cast`/`br_on_cast_fail`
(structured control flow that branches based on a dynamic type test —
genuinely new control-flow shape, not just a new opcode), and recursive/
mutually-recursive type definitions (`type-rec.wast`, `type-canon.wast`
in the vendored corpus's upstream listing — WASM06's own type checker
would need real type-equivalence-under-recursion logic, not just index
lookups).

### Recommended shape if picked up

`funcref`/`externref` as real value types (closing the gap that today
makes `global.wast`, `select.wast`, `br_table.wast`, `call_indirect.wast`
all fail to parse in the vendored corpus — confirmed via this session's
own WASM08 investigation) is the natural first slice: high conformance
value (4+ already-vendored MVP-adjacent files unlock immediately), and
doesn't require the harder `br_on_cast`/recursive-type pieces. Array
types and the `br_on_*` control-flow family are a clearly separable L
slice after that.

---

## Epic 5: The Component Model

**Size: XL**, and arguably out of scope for this repo's architecture
entirely, at least in its current form.

### What it is

Not an instruction-set extension at all — a completely different binary
container format layered on top of core WASM modules, with its own type
system (records, variants, resources, interfaces), a "canonical ABI" for
marshalling between component-model types and core-WASM linear memory,
and a component-linking model distinct from module-linking.

### Why this is different from every other epic here

Every other epic in this document is "more opcodes, one more value type,
maybe one more control-flow shape" — additive to the existing
decode-a-byte-stream-into-typed-operations architecture this whole WASM
stack (`wasm-opcodes` → `wasm-module-parser`/`wasm-wast-parser` →
`wasm-validator` → `wasm-execution`) is built around. The component
model is not that: it's a second file format that *contains* core
modules rather than extending them, with a type system that has no
analogue in `wasm_types::ValueType` at all (records/variants/resources
aren't expressible as WASM numeric/reference types).

### Recommendation

Do not schedule this without a dedicated scoping investigation first
(more scoping than this document attempts) — the honest assessment is
that it may warrant treatment as an entirely separate crate family
(`wasm-component-*`) built *on top of* the existing core-WASM stack
rather than an extension of it, closer in shape to how `wasm-runtime`
sits on top of `wasm-execution` today than to how, say, SIMD would slot
into the existing opcode tables.

---

## The JIT Tier

**Size: XL**, and currently blocked on a prerequisite that doesn't exist.

### The blocker

`jit-core` (confirmed via reading its own module doc comment) is built
around `interpreter-ir` (`IIRModule`/`IIRFunction`/`IIRInstr`) — the IR
this repo's *other* language frontends (Tetrad, BASIC, Python) already
lower to before `jit-core`'s `specialise()`/`CIROptimizer`/`Backend`
pipeline can touch them. `wasm-execution` does not lower WASM bytecode to
IIR at all — it's a tree-walking interpreter that decodes and dispatches
raw WASM opcodes directly (`decode_function_body` → per-opcode handler
closures registered on `GenericVM`). There is currently no
`wasm-to-iir` lowering pass anywhere in this repo.

This matches this session's own earlier-established finding (carried
forward from before this document): the JIT tier is blocked on a
currently-nonexistent WASM→IIR lowering pass, and that remains true as
of this write-up — nothing in WASM04 through WASM08 changed it.

### What building the prerequisite would look like

A new `wasm-to-iir` crate, structurally parallel to how the OTHER
frontends (Tetrad/BASIC/Python, whichever already exists) lower to IIR
today — read one of those for the actual established pattern before
starting, rather than inventing a new one. WASM's own structured control
flow (`block`/`loop`/`if`, branch depths resolved statically) maps
reasonably cleanly onto IIR's own control-flow representation (this
repo's `interpreter-ir` crate already handles arbitrary control flow for
other source languages, so this isn't a novel problem, but it is real
translation work: every opcode family in `wasm-opcodes`' table needs an
IIR-instruction mapping, and the multi-value/branch-arity semantics
WASM04/WASM06 just finished nailing down for the tree-walking
interpreter need an equivalent, independently-correct encoding in
IIR-land).

### Recommendation

Do not start this speculatively. `jit-core`'s existing `Backend` trait
being reused across multiple source languages *is* real, useful
precedent (confirmed, not assumed) that a WASM backend wouldn't need a
whole new JIT engine, just the lowering pass — but that lowering pass is
itself an L-or-XL-sized project on its own, independent of whether a JIT
is ever attached to it. If a `wasm-to-iir` lowering pass is ever built
for *other* reasons (e.g. wanting to run WASM's own semantics through
the same optimizer other languages get), the JIT tier becomes a much
smaller follow-on. Building it *only* to enable the JIT is the expensive
order; building it because IIR-level tooling (the optimizer, whatever
static analysis exists for other frontends) becomes independently
valuable for WASM is the cheap order.

---

## Suggested sequencing (opinion, not a commitment)

1. **Tail calls** (S) — cheapest real win, clean scope, natural next
   pick after this session's run of merged WASM PRs.
2. **`funcref`/`externref` as real value types** (the first slice of
   Epic 4) — unlocks several already-vendored, already-failing-to-parse
   corpus files immediately, moderate size.
3. **Plain atomics without real concurrency** (the first slice of Epic
   2) — real conformance value, doesn't block on the threading
   architecture question.
4. **SIMD** — largest opcode count but mechanically the most repetitive;
   good candidate for a multi-session, multi-PR arc once someone commits
   to it, following the spec-first two-stage shape this session used for
   WASM04.
5. **Everything else** (full GC, real threading/concurrency, the
   component model, the JIT tier) — each needs its own dedicated
   scoping session before a size estimate here should be trusted much
   further than "XL."

This ordering optimizes for "real, verifiable conformance-corpus wins
per unit of implementation risk," matching the pattern this session's
own WASM04/WASM06/WASM08 arc already validated: pick the smallest thing
that unlocks real, currently-blocked corpus coverage, verify via the
full baseline diff, ship it, repeat.
