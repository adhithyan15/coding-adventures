# W20 — GC epic continuation, first slice: real conformance for the existing `i31ref` implementation

## Purpose and how this slice was chosen

`code/specs/W07-wasm-post-mvp-epics.md` (the original "plan the opcode-family
epics" survey) leaves four things un-started after WASM16 (tail calls), WASM17
(funcref/externref), WASM09/W09 (plain atomics), and the SIMD + relaxed-SIMD
arc (W13-W19): full GC beyond this repo's existing narrow struct/i31 slice,
real threading (`memory.atomic.wait32`/`wait64`/`notify`), the component
model, and a JIT tier. This spec picks up the first of those, but the actual
first slice below is **narrower** than the "array types +
`br_on_null`/`br_on_non_null`/`br_on_cast`/`br_on_cast_fail`" scope a prior
session's lightweight scan suggested — the rest of this section explains why,
with evidence, not just an opinion.

### Re-checking the other candidates first (a real prioritization pass, not a rubber stamp)

- **Real threading (`wait`/`notify`)**: still architecturally blocked, and
  WASM10/W12 (dedicated-thread `call_function`) does **not** change that.
  Re-reading W12 confirms its thread is spawned and immediately, synchronously
  `.join()`-ed before the calling thread does anything else — "no two threads
  ever run concurrently," by its own design. `wait`/`notify` need a second
  *actually-concurrent* agent to notify a blocked one; W12's stack-size trick
  gives WASM a bigger stack, not a second live thread. W09's own "explicitly
  out of scope" section already says as much. Not a viable first slice.
- **Component model**: W07 already recommends "do not schedule without a
  dedicated scoping investigation first" — unchanged, not revisited here.
- **JIT tier**: still blocked on a nonexistent `wasm-to-iir` lowering pass —
  unchanged, not revisited here.
- **Tail calls / funcref+externref / plain atomics / SIMD+relaxed-SIMD**: done.

So GC continuation is still the right epic — every other candidate is either
already finished or genuinely blocked/XL with no small entry point. The
question this spec actually had to answer is *which part* of GC.

### The GC/reference-types corpus is far more cross-entangled than it looks from the outside

Direct evidence gathered against the pinned `WebAssembly/testsuite` commit
this whole campaign uses (`28864811cf03bdbf880733786148feaba339582d`, same
repo, no new upstream needed — confirmed via `gh api
repos/WebAssembly/testsuite/git/trees/<sha>`):

- `br_on_null.wast` and `br_on_non_null.wast` (the two opcodes a prior
  session's scan named) — **every single test module in both files calls
  `call_ref`**, a function-references-proposal opcode this repo doesn't
  have, and declares locals/params with genuinely **non-null** concrete
  function-type references (`(ref $t)`, distinct from `(ref null $t)`) that
  this repo's type system doesn't model at all (`wasm_types::ValueType` has
  no non-null/nullable distinction anywhere). Implementing `br_on_null`/
  `br_on_non_null` well enough to pass any of these files' real directives
  means also implementing `call_ref` and a non-null-ref type story — not a
  "2 opcodes" slice.
- `ref_as_non_null.wast` — same `call_ref` + non-null-ref dependency.
- `struct.wast` — depends on `(rec ...)` recursive type declarations,
  forward type references, packed (`i8`/`i16`) struct fields, and
  multi-field structs mixing numeric/reference field types — well beyond
  this repo's existing narrow single/few-field `struct.new`/`get`/`set`
  slice.
- `array.wast`/`array_copy.wast`/etc. — a wholly new heap-object kind
  (mutable-length arrays), plus `array.new_data`/`array.new_elem`
  interacting with data/elem segments.
- `ref_eq.wast` — needs `(sub (struct ...))` subtype declarations, the
  abstract `eq`/`any`/`none` heap-type hierarchy, and `array.new_default`.
- `br_on_cast`/`br_on_cast_fail` — need real runtime type-testing across
  that same heap-type hierarchy (this repo's `ref.test` slice is narrow —
  see below).

None of this is separable into a clean 2-opcode first bite. The real
upstream testsuite was written assuming the *entire* GC + function-
references + typed-references baseline already exists together, so almost
every file pulls in a chunk of all of it.

### What actually is separable: `i31.wast`

`i31.wast` (2955 bytes at the pinned SHA) is the one GC-family file that is
**not** entangled with `call_ref`, non-null concrete refs, `(rec ...)`
declarations, or the `eq`/`any`/`none` abstract-heap-type hierarchy. Its
first module (the one this slice targets — see "Scope" below) only needs:
`(ref i31)`/`(ref null i31)` as a value type in params/results/locals/
globals, `ref.i31`, `i31.get_s`, `i31.get_u`, and `ref.null i31`. This repo
already has a **real, working, but never-conformance-tested** i31 slice
(`struct.new`/`get`/`set`, `i31.new`/`get_s`, `ref.test` — built for the
LANG77 Lisp `cons`/`car` compiler, LANG77 L3b-3a-3a/L3b-3a-3b) — it has
**never once been run against the real WebAssembly test corpus**, only
against hand-built binary bytecode in this repo's own unit tests. That gap
is itself worth closing: "compiles, has unit tests" is not the same claim as
"passes the real spec's own conformance suite," and this repo's own working
principles treat that distinction as load-bearing.

Investigating `i31.wast` closely enough to vendor it for real also surfaced
a genuine correctness bug in the existing slice (see "Bug: `i31.new`/
`i31.get_s` were stack-identity no-ops" below) — exactly the kind of gap
that never shows up until the real corpus is run.

## Scope

### In scope

1. **Fix `ref.i31` (the WasmGC sub-opcode `0xFB 0x1C`, this repo internally
   called `i31.new`) to actually mask its `i32` operand to 31 bits**, per
   spec (`i31ref` carries only the low 31 bits of its `i32` operand).
2. **Fix `i31.get_s` (`0xFB 0x1D`) to actually sign-extend from bit 30**,
   and to trap on a null i31 reference.
3. **Add `i31.get_u` (`0xFB 0x1E`)**: zero-extend (mask to 31 bits), trap on
   null.
4. **`wasm-wast-parser` text support** (none of this existed before this
   slice — this repo's whole existing struct/i31 GC slice has *zero* wast
   text syntax, only hand-built binary bytecode):
   - `parse_value_type`: `(ref i31)`, `(ref null i31)`, and the bare
     `i31ref` keyword, all mapping to `ValueType::I31ref`. Null vs.
     non-null is **not** distinguished (same simplification this crate
     already makes for `funcref`/`externref` — see its own doc comments) —
     an explicit, deliberate choice, not an oversight.
   - `parse_ref_null_heap_type`: recognize `i31` (byte `0x6C`).
   - Instruction names `ref.i31`, `i31.get_s`, `i31.get_u` in both the
     flat/stream and folded encoders (`encode_stream_instr`/
     `encode_flat_instr`), the same "intercept by name before
     `wasm_opcodes::get_opcode_by_name`" shape `ref.null`/`ref.is_null`
     already use, since none of the `0xFB`-prefixed GC family is (or needs
     to be) registered in `wasm_opcodes::OPCODES`.
5. **`evaluate_const_expr`** (`wasm-execution`'s separate, restricted
   constant-expression evaluator, used for global initializers — genuinely
   different code from the main `GenericVM` interpreter): add `0xFB 0x1C`
   (`ref.i31`) support, needed by `i31.wast`'s `(global $i (ref i31)
   (ref.i31 (i32.const 2)))`.
6. **`wasm-validator`**: add a type rule for `i31.get_u` (mirrors the
   existing `i31.get_s` rule exactly: pop, push `I32`), and map
   `ref.null`'s heap-type byte `0x6C` to `ValueType::I31ref` instead of
   falling back to `Unknown` (a small precision improvement, same shape as
   the existing `0x70`/`0x6F`/`0x0F` cases).
7. **Vendor `i31.wast` verbatim** (pinned SHA
   `28864811cf03bdbf880733786148feaba339582d`, same repo every other file
   in this campaign uses) into `wasm-conformance`, add it to
   `TESTSUITE_FILES`, regenerate the baseline.

### Explicitly out of scope (this slice)

- **`call_ref`, non-null concrete reference types (`(ref $t)` for a
  concrete `$t`), `(rec ...)` recursive type declarations** — the reason
  `br_on_null`/`br_on_non_null`/`ref_as_non_null.wast`/`struct.wast` aren't
  in this slice (see above). A future slice.
- **Tables/elem-segments of non-`funcref` reference kind** (`i31.wast`'s
  later modules: `(table $table 3 10 i31ref)`, `(elem ... i31ref (item
  (ref.i31 ...)) ...)`). This repo's element-segment representation
  (`ctx.elements: Vec<Vec<Option<u32>>>`) is function-index-shaped; storing
  arbitrary GC handles there is a real, separate generalization, not a
  consequence of fixing `i31.get_u`. Per W14 (per-module build failures are
  captured, not fatal to the whole file — confirmed still true by reading
  `wasm-wast-parser::script::Directive::Module`'s `Result`-wrapped shape),
  vendoring `i31.wast` wholesale is still safe: these later modules will
  build-fail and grade `NotYetSupported` for their own directives, without
  affecting the first module's real, passing grade.
- **`ref.cast`** (`0xFB 0x16`) — used by `i31.wast`'s later
  `anyref`-global/table modules, not by the in-scope first module.
- **Array types, `br_on_cast`/`br_on_cast_fail`, the `eq`/`any`/`none`
  abstract heap-type hierarchy, real threading, the component model, the
  JIT tier** — unchanged from W07's own assessment.

## Bug: `ref.i31`/`i31.get_s` were stack-identity no-ops, not real box/unbox

`wasm-execution/src/lib.rs`'s `0xFB` handler had `0x1C | 0x1D => {}` — i.e.
literally nothing: the `i32` value already on the stack from whichever
instruction pushed it passes straight through unmodified for **both**
`ref.i31` (0x1C) and `i31.get_s` (0x1D). The accompanying comment says
"`i31ref` ≡ its `i32` payload — stack-identity," which is true only for the
specific values this repo's own LANG77 Lisp-compiler tests exercise (small
positive integers like 7, 9, 42 — comfortably inside 31 bits either way, and
where sign-extension from bit 30 is a no-op since bit 30 is 0).

The real spec's semantics are only equivalent to identity in that narrow
case. Confirmed against `i31.wast`'s own real test vectors:
`i31.get_s(i32.const 0x7fff_ffff)` must be `-1` (`0xFFFFFFFF`) — a pure
identity pass-through would wrongly return `0x7fff_ffff` (positive, bit 31
never set). The fix:

- `ref.i31` masks its operand to 31 bits: `v & 0x7FFF_FFFF`.
- `i31.get_s` sign-extends from bit 30: `(v << 1) as i32 >> 1` (arithmetic
  shift), after checking for a null reference (trap: `"null i31
  reference"`).
- `i31.get_u` (new) masks to 31 bits (a no-op if the value already went
  through `ref.i31`'s masking, but implemented as an explicit, independent
  mask rather than relying on that invariant — defense in depth, and
  correct even if a value reaches `i31.get_u` some other way), same
  null-check.

Every existing hand-built-bytecode test in `wasm-execution` (the LANG77
`cons`/`car` round-trips) only ever boxes/unboxes small positive integers,
so this fix changes no existing test's expected output — confirmed by
running the full existing suite unchanged before vendoring `i31.wast`.

## Verification plan

- Unit tests for `ref.i31`'s masking, `i31.get_s`'s sign-extension (using
  the exact `0xaaaa_aaaa`/`0xcaaa_aaaa`/`0x7fff_ffff` vectors `i31.wast`
  itself uses — verified against the real file, not invented), `i31.get_u`'s
  zero-extension, and both opcodes' null-reference trap.
- Unit tests for the new `wasm-wast-parser` value-type and instruction-name
  parsing (`(ref i31)`, `(ref null i31)`, `i31ref`, `ref.i31`, `i31.get_s`,
  `i31.get_u`, in both flat and folded form).
- Vendor `i31.wast`, regenerate `wasm-conformance`'s baseline, and diff
  against the pre-change baseline: zero regressions on any already-parsing
  file, plus a real, non-zero `assert_return`/`assert_trap` pass count on
  the new file's first module (`new`/`get_u`/`get_s`/`get_u-null`/
  `get_s-null`/`get_globals`/`set_global` — 20 directives). Later modules in
  the same file are expected (and fine) to grade `NotYetSupported`, not
  `Fail` — see "Explicitly out of scope" above.
- `/security-review` before push, per this repo's standing workflow.
