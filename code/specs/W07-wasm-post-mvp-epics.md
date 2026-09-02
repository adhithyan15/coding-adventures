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

---

## Addendum (2026-09-01) — known remaining gaps found during a fresh
## corpus-wide prioritization pass, after W32/W33/W34 closed

Everything above is stale (SIMD/atomics/tail-calls/GC-beyond-the-narrow-
slice have all since shipped — see `W08`-`W34`). Rather than rewrite this
whole survey, this addendum records what a full `wasm_conformance_report`
run against the pinned 257-file corpus turned up as genuinely still open,
for whoever does the next prioritization pass. Two real regressions found
in the same pass (a `wasm-validator` funcref-assignability bug wiping out
`br_table.wast`, and a mutable-global/elem-vs-data-ordering bug in
`wasm-runtime`) were already fixed (see `wasm-validator`/`wasm-runtime`/
`wasm-conformance` CHANGELOGs, PRs #13881/#13882). What's left:

1. **~~Cross-instance function-reference identity (the big one).~~
   CLOSED (2026-09-01) — see `W35-wasm-cross-instance-function-identity.md`
   and its own closing addendum.** `wasm-execution`'s table entries stored
   bare `u32` function indices with no instance identity attached. When a
   table (or an element/global of funcref type) is shared across
   module-linking boundaries via `register`/import, `call_indirect`/
   `table.get` resolved that index against whichever instance happened to
   be EXECUTING right now, not whichever instance actually wrote it. This
   was confirmed (not guessed) as the sole remaining cause of every real
   (non-"not yet supported") failure in `elem.wast`, `linking.wast`,
   `linking0.wast`, and `linking3.wast` as of this pass — e.g.
   `linking.wast` expected `4` and `-4` from two adjacent calls and got
   them swapped, a classic wrong-instance-namespace symptom. `W35` shipped
   as four dependency-ordered slices (PRs #13900/#13908/#13915, plus the
   fourth slice closing the epic): `FuncRefTarget`/`func_ref_heap` (a
   `Copy` handle over a non-`Copy`, self-contained, cross-instance-safe
   payload — NOT the originally-sketched bare `Rc<WasmInstance>` embedded
   in `WasmValue`, which its own spec found unbuildable as stated; see
   that document's own "Why the naive `Rc<WasmInstance>` sketch doesn't
   work as stated" section), `WasmInstance::func_identities`/
   `instance_identity`, and a `wasm-conformance`-driven resolution fixup
   pass run once per registered module. **Outcome**: `linking.wast`
   (55/65 → 65/65), `linking0.wast` (0/1 → 1/1), and `linking3.wast`
   (5/6 → 6/6) all reached full pass; `elem.wast` reached 18/19 (one
   remaining failure confirmed to be a pre-existing, W35-unrelated
   externref bug, not part of this item's own scope). One genuine,
   documented follow-on gap remains open: funcref-typed GLOBALS still
   lack real cross-instance resolution (not needed by any current corpus
   case) — see W35's own closing addendum for the concrete blocker
   (a `return_call_ref.wast` regression) and what a future fix needs.
   **`elem.wast`'s own remaining failure since CLOSED (2026-09-01)** — see
   item 3's own updated entry below (found and fixed alongside the
   `table.wast` investigation): a `wasm-wast-parser` restriction, not a
   table-identity bug, was the real cause — `elem.wast` now passes
   19/19, and the whole corpus's `assert_return` category is at 100%.

2. **Malformed-binary LEB128 under-strictness.** `binary-leb128.wast`
   (7 real failures) and `binary.wast` (2 real failures) each contain
   `assert_malformed` cases where `wasm-module-parser` accepts a binary
   module it should reject (over-long/non-canonical LEB128 encodings,
   confirmed via a direct probe — every failure is "binary module parsed
   but should have been rejected as malformed"). Contained to
   `wasm-module-parser`'s LEB128 decoding, no representation questions —
   a clean, self-contained next PR. Sized **S**.

3. **`table.wast`'s oversized-declared-minimum case — investigated and
   FIXED (2026-09-01).** `table.wast` line 9, `(module definition (table
   0xffff_ffff funcref))` (`u32::MAX`, no declared `max`), is a bare,
   UNWRAPPED directive — no `assert_invalid` around it — so the official
   testsuite itself asserts this declaration must VALIDATE.
   `wasm-validator` was rejecting it for exceeding this interpreter's
   10,000,000-element resource-limit heuristic (`ValidationError: table
   #0: declared minimum 4294967295 elements exceeds this interpreter's
   resource limit of 10000000`).

   Confirmed against the real spec's own implementation-limits reasoning:
   a 32-bit table's `min` has no spec ceiling below `2^32 - 1` — an
   implementation MAY refuse to actually ALLOCATE an oversized table, but
   may NOT refuse to let a module merely DECLARE one. This is exactly the
   same "declare freely, cap only at real allocation time" treatment
   `is64` tables and 64-bit memories already received elsewhere in
   `wasm-validator`; the 32-bit table case was the one remaining
   inconsistency, confirmed as a genuine conformance bug, not a
   defensible implementation choice.

   **Fix**: removed the structural-validation-time rejection (`wasm-
   validator` 0.2.85's old Check 2b, both the per-table and cross-table-
   aggregate halves) entirely; the practical resource-limit heuristic
   moved to `wasm-runtime::instantiate` (0.6.28) — the pipeline stage
   where real allocation actually happens — generalizing that crate's
   pre-existing `is64`-only aggregate cap to cover every table. Verified
   not a DoS regression: `wasm-execution::Table::new_with_is64` already
   applied its own per-table cap unconditionally regardless of `is64`
   (0.9.91's doc-only fixup), so only the aggregate half needed a new
   home, not new protection. See `wasm-validator`/`wasm-runtime`/
   `wasm-execution`/`wasm-conformance`'s own CHANGELOGs for the full
   writeup and corpus-wide verification (exactly 3 files changed in the
   whole 257-file corpus across both this fix and the `elem.wast` one
   above, every other file byte-identical).

---

## Addendum 2 (2026-09-02) — path to literal 100% corpus conformance

Prompted directly by a user question ("what's preventing 100%?") after the
W35 epic closed (PRs #13900/#13908/#13915/#13924) and the three items in
Addendum 1 were resolved or dispatched. This addendum is the first
genuine, per-file-grounded survey of what "the remaining `not_yet_
supported` directives" actually consist of — Addendum 1 only ever dealt
with real, gradable FAILURES; this one is about the much larger bucket of
directives this interpreter has never attempted at all.

### The real numbers (regenerate before trusting — this drifts fast)

`cargo run --release --bin wasm_conformance_report -p wasm-conformance --`
as of the PR immediately following #13924:

```
module               1982/1983 (99.9%), 270 not yet supported
register             73/73 (100.0%), 5 not yet supported
action               357/357 (100.0%), 58 not yet supported
assert_return        51778/51779 (100.0%), 985 not yet supported
assert_trap          2970/2970 (100.0%), 1998 not yet supported
assert_exhaustion    5/5 (100.0%), 10 not yet supported
assert_invalid       2667/2667 (100.0%), 95 not yet supported
assert_malformed     1313/1313 (100.0%), 627 not yet supported
assert_unlinkable    254/254 (100.0%)
assert_exception     18/18 (100.0%)
```

**4048 total `not_yet_supported` directives across 79 of 257 files** (a
Python one-liner summing `not_yet_supported` per file across `tests/
fixtures/testsuite-status.json`'s own `files` dict — reproduce this
yourself before trusting the count below, it moves with every merge).
Every 100.0% figure above still has a nonzero NYS count hiding inside
it — "100%" here means "100% of what was actually GRADED," never "100%
of the file."

### Correction (2026-09-02, see `W36-wasm-element-segment-exprs-list.md`)

The table below's headline attribution — that the exprs-list element-
segment form causes the 2130-directive `table_copy*`/`table_init*.wast`
bucket — turned out to be WRONG, caught by `W36`'s own spec-writing pass
before any implementation work was wasted chasing it. The real root
cause there is unrelated to element segments at all: `wasm-wast-parser`
doesn't implement the spec's own `table.init y ≡ table.init 0 y` /
`table.copy ≡ table.copy 0 0` table-index-elision abbreviation, so every
module using that shorthand (which is all four of these files) fails to
parse for a completely different reason. The genuine exprs-list gap is
real but much smaller (~15-20 directives, confined to `elem.wast`/
`global.wast`/`ref_func.wast`) — see `W36` for the corrected, spec-text-
grounded picture and the actual slice plan. The table and "Recommended
sequencing" list below are left AS ORIGINALLY WRITTEN (not retroactively
fixed) so this addendum stays an honest record of what was believed at
the time — read them as history, not as the current plan.

### Per-file NYS breakdown, sorted by weight (the actual backlog)

The single most important finding: **this is NOT dominated by whole
undelivered proposals** (contra what a naive read of "SIMD/GC/exceptions
still incomplete" might suggest) — most `simd_*.wast` files carry exactly
**one** NYS directive each. The weight is concentrated in a small number
of files gated by a SMALL number of shared root causes, discovered by
direct probing (`wasm_conformance::run_wast_source` on each file, reading
every distinct `DirectiveOutcome::NotYetSupported` message):

| Files | NYS directives | Shared root cause (verified by direct probe, not guessed) |
|---|---|---|
| `table_copy.wast`, `table_copy64.wast`, `table_init.wast`, `table_init64.wast` | 566+566+499+499 = **2130** (53% of ALL remaining NYS) | **The "exprs-list" active/declarative element-segment form** — both a TEXT-grammar gap (`wasm-wast-parser`: "expected an active segment to use a plain function-index list instead (exprs-list is only supported for passive segments)") and a BINARY-decoding gap (`wasm-module-parser`: "unsupported element segment mode flags 3/4/6/7 (only 0/1/2/5 supported)" — flag bits 3/4/6/7 are exactly the bulk-memory-proposal's `exprs`-typed active/declarative segment encodings). These 4 files' own top-of-file modules use this form pervasively for setting up their bulk-copy/bulk-init test fixtures — ONE module failing to parse cascades into hundreds of downstream `assert_trap`/`assert_return` NYS. **This is the single highest-leverage item in the entire remaining backlog.**
| `elem.wast` | 67 (down from more before Addendum 1's fixes) | The SAME exprs-list gap (most of its own NYS module failures cite the identical error strings), plus a smaller, distinct "table with an explicit init expression (function-references proposal) is not yet supported" gap, plus a couple of unrelated `unknown table identifier "$e"` cases worth a second look once the exprs-list fix lands (they may just be a downstream symptom of the same root parse failure, or a real second bug — undetermined). |
| `select.wast` | 126 | A SINGLE contained text-parser gap — `wasm-wast-parser` doesn't recognize `result` as a valid folded-instruction-position token for `select`'s own typed form (`(select (result funcref) ...)`), confirmed via probe ("unknown instruction 'result'"). Only 2 actual parse failures in the whole file, but they gate ~124 downstream `invoke`/`assert_return`/`assert_trap` directives that reference the broken modules. High leverage-to-effort ratio — likely a small, self-contained `wasm-wast-parser` grammar fix. |
| `global.wast` | 71 | Two DISTINCT causes: (1) an unresolved `spectest.global_i32` import (this corpus's harness doesn't wire up the informal `spectest` host module some upstream testsuite files assume — check whether OTHER already-100%-passing files already solve this via a real `spectest` host stub `wasm-conformance` provides, or whether this is a new, small piece of harness work), and (2) the SAME exprs-list active-segment gap as above. |
| `imports.wast`, `imports1-4.wast` | 75+5+11+6+5 = 102 | Not yet individually probed — grouped here as "linking/import edge cases," needs its own investigation pass before sizing. |
| GC proposal remainder: `ref_eq.wast`, `ref_test.wast`, `ref_cast.wast`, `i31.wast`, `br_on_cast.wast`, `br_on_cast_fail.wast`, `br_on_null.wast`, `br_on_non_null.wast`, `array*.wast` (7 files), `struct.wast`, `type-subtyping.wast`, `type-rec.wast`, `table-sub.wast`, `ref.wast`, `ref_func.wast`, `ref_is_null.wast`, `ref_as_non_null.wast`, `call_ref.wast`, `return_call_ref.wast` | ~500 combined | The genuinely-remaining slice of the GC proposal beyond what W20/W32-W35 already closed — likely still gated significantly by the SAME exprs-list gap for these files' own elem-segment-heavy GC test fixtures (unconfirmed — re-probe each file AFTER the exprs-list fix lands before scoping further GC-specific work, since a large fraction of this bucket may simply evaporate). |
| `return_call.wast`, `return_call_indirect.wast` | 35+64 = 99 | Tail-call (W11) remainder — not yet individually probed. |
| `bulk.wast`, `memory_copy0/1.wast` | 42+29+14 = 85 | Bulk-memory-proposal edge cases — not yet individually probed, may also share the exprs-list root cause given the family resemblance to `table_copy`/`table_init`. |
| `utf8-invalid-encoding.wast` | 176 (100% of the file) | A genuinely separate, self-contained gap: this file's `assert_malformed` cases test INVALID UTF-8 byte sequences in name/custom-section strings specifically — needs real UTF-8 validation somewhere in `wasm-module-parser`'s string-decoding path (a well-defined, standalone **S**-sized item, no dependency on anything else in this table). |
| `simd_const.wast`, `simd_align.wast`, various `simd_load*_lane.wast`, `float_literals.wast`, `int_literals.wast`, `const.wast`, `if.wast`, `loop.wast`, `block.wast`, `align.wast`, `align64.wast`, `token.wast`, `annotations.wast`, `func.wast`, `call_indirect.wast` | ~500 combined, mostly 1-46 each | Long tail of small, mostly `assert_malformed`/`assert_invalid` text-format-parser edge cases (custom annotations syntax, specific numeric-literal malformed forms, alignment-immediate malformed forms) — each individually small, S-sized, not yet triaged for shared root causes.
| Everything else (~60 files) | ≤20 each, mostly ≤5 | Long tail, not worth a table row — re-probe individually if pursuing literal 100%.

### Recommended sequencing

1. **The exprs-list active/declarative element-segment form** (text
   grammar in `wasm-wast-parser` + binary flag-bit decoding in
   `wasm-module-parser`, likely also touching `wasm_types::Element`'s
   representation to carry inline expressions rather than bare function
   indices, which would then ripple into the elem-segment-application
   code W35 just finished building in `wasm-runtime`/`wasm-execution`).
   **Do this first** — it's the single highest-leverage item by a wide
   margin (a plausible 2000+ of the 4048 total NYS directives gate on
   it, once `table_copy*`/`table_init*`/`elem.wast`/likely-`bulk.wast`
   and a meaningful fraction of the GC-remainder bucket are re-probed
   after the fix lands). Sized **L** (a real grammar/binary-format
   extension plus a representation change touching multiple crates,
   comparable to W32/W33's own sizing) — warrants its own spec-first PR
   (next free W-number) given the surface area, following this
   campaign's established convention.
2. **`select.wast`'s typed-result folded-form parsing gap.** Small,
   contained, high leverage-to-effort ratio. Sized **S**.
3. **`utf8-invalid-encoding.wast`'s real UTF-8 validation gap.**
   Self-contained, no dependency on item 1. Sized **S**.
4. **`global.wast`'s `spectest.global_i32` import gap** — check first
   whether this is actually a missing, cheap, one-time host-stub
   addition to `wasm-conformance`'s own test harness (likely) rather
   than an interpreter capability gap at all. **DONE** (`wasm-conformance`
   0.1.123): it was exactly a harness gap, and a live census found the
   corpus actually needs the ENTIRE upstream `spectest` module (13
   exports across 23 files), not just `global_i32` — see that crate's own
   CHANGELOG for the full census and per-file diff. Closed 272 `not_yet_
   supported` directives across 18 files with zero regressions; no
   changes needed in any interpreter crate.
5. **Re-probe everything** after items 1-4 land — several of the
   still-untriaged buckets above (GC remainder, bulk-memory, imports)
   may substantially shrink once the exprs-list fix removes their
   shared upstream blocker, and re-probing BEFORE writing new specs
   for them avoids scoping work against a stale picture.
6. **Long-tail S-sized text-format-parser gaps** (numeric literals,
   annotations, alignment immediates) — pick off individually once the
   high-leverage items are closed; each is small enough not to need its
   own spec, per this repo's own proportionality principle.

### An honest caveat literal 100% will still not clear

Even closing every item above only gets the corpus to "100% of what
this interpreter attempts to grade." A handful of `assert_exhaustion`/
`assert_invalid` NYS entries (e.g. `skip-stack-guard-page.wast`'s 10
`assert_exhaustion` cases) may depend on this interpreter's own stack-
depth/resource-limit architecture in ways that are legitimately harder
than a parser/grammar fix — re-scope those individually when reached
rather than assuming this table's sizing guesses hold all the way down.
