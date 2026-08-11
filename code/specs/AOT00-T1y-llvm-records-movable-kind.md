# AOT00-T1y — LLVM records/unions join the movable GC kind

> Status: **landed** (`iir-to-llvm` 0.53.0). Closes the one item
> `AOT00-T1w-llvm-gc-completion.md` §5 explicitly scoped out: *"A per-kind
> precise interior trace for LLVM's `alloc`'d objects — same kind-0/
> conservative boundary `AOT00-T1v` draws for `vm-core`; LLVM shares the
> identical `gc-core-capi` engine, so the same boundary applies unchanged."*
> `AOT00-T1v` (`vm-core`) closed its half of that boundary this session via
> `AOT00-T10`'s tagged-kind mechanism. This spec closes the other half —
> LLVM — using the mechanism the native backends already use, not a new one.

## 1. The gap, precisely

Every Twig record/union constructor erases (frontend, `twig-ir-compiler`) to
a `ref<LispyPair>` cons cell built with the structural `alloc`/`field_store`
IIR ops. Three backends lower `alloc`:

- **`aarch64-backend`/`x86_64-backend`** (native AOT): `alloc` with no
  explicit size (or explicit size `16`) calls `__twig_gc_alloc_pair()` — a
  **movable**, precisely-traced `{0,8}` kind. Confirmed at
  `aarch64-backend/src/lib.rs:1494-1520` and the `x86_64-backend` mirror.
- **`iir-to-llvm`**: `lower_alloc` (`iir-to-llvm/src/lib.rs:2459`)
  unconditionally calls `@__twig_gc_alloc(i64 <size>)` — the **kind-0,
  conservative, pinned** allocator — regardless of size.

Both call into the exact same `gc-core-capi` engine (`gc-core-capi/src/
twig_compat.rs`), so this isn't a missing GC feature — it's LLVM simply
never being wired onto a primitive that's already implemented, tested, and
shipped for the identical object shape on the native backend. Practical
effect: a record/union compiled through LLVM is correctly collected
(nothing leaks — kind-0 marking is a sound, if imprecise, over-
approximation) but never eligible for relocation under a compacting
collection, and its two fields are conservatively scanned rather than
precisely traced.

## 2. The fix — reuse `__twig_gc_alloc_pair`, don't invent anything

`gc-core-capi::__twig_gc_alloc_pair()` (`twig_compat.rs:85`) already exists,
is already the native backends' production path, and needs zero changes:
it lazily registers a `{0,8}` tagged... no — **boxed** kind (native's
object model guarantees every field of a record/union constructor cell is
boxed, per `twig-ir-compiler`'s `emit_record_def` typing every constructor
param `any` — this is `register_kind`'s boxed mode, not `AOT00-T10`'s new
tagged mode; the two features are unrelated despite both closing a
"kind-0 conservative" boundary this session) via `__gc_register_kind`,
caches the kind id in a `static`, and returns a fresh, movable, precisely-
traced 2-word cell on every call. No arguments, no size parameter — it
*is* the pair shape.

`iir-to-llvm::lower_alloc` gets the identical branch `aarch64-backend`
already has (`aarch64-backend/src/lib.rs:1494-1520`, quoted in full since
this spec's implementation is a direct port):

```rust
match explicit_size {
    None | Some(16) => {
        // movable {0,8} pair, no arg
    }
    Some(size_bytes) => {
        // kind-0 conservative fallback, unchanged
    }
}
```

Ported to LLVM IR text emission:

```llvm
; default / explicit 16-byte alloc:
%dest = call i64 @__twig_gc_alloc_pair()

; any other explicit size (unchanged):
%dest = call i64 @__twig_gc_alloc(i64 <size>)
```

New declare, emitted only when at least one `alloc` in the module takes
this branch (mirrors the existing `used_gc_alloc`-gated declare pattern for
`@__twig_gc_alloc` at `iir-to-llvm/src/lib.rs:880-884`):

```llvm
declare i64 @__twig_gc_alloc_pair()
```

`@__twig_gc_alloc`'s existing declare stays, gated on whether any `alloc`
in the module still takes the non-pair (explicit, non-16 size) branch —
today that's `alloc_bytes`-adjacent LANG-FULL E5 array support and any
future non-default-size `alloc` caller; Twig's own record/union/cons/
closure emission always uses the no-operand/16-byte default, so a
Twig-only module will use `@__twig_gc_alloc_pair` exclusively and never
declare (or link) `@__twig_gc_alloc` at all — harmless, since an unused
`declare` with no call site is legal LLVM.

## 3. Why this needs no write-barrier change

`field_store` (`lower_field_store`, `iir-to-llvm/src/lib.rs:2472`) already
calls `@__twig_gc_write_barrier` unconditionally on every store — this
predates and is independent of which allocator produced the object being
stored into. A pair allocated via `__twig_gc_alloc_pair` is barrier-covered
by the exact same code path a kind-0 `__twig_gc_alloc` object already was.
Nothing here changes.

## 4. Why this needs no `field_load`/`field_store` change

Both already treat a field as a raw 64-bit word at `ptr + idx*8`
(`getelementptr i64, ptr, i64 <idx>`) — that's the pair kind's own layout
(`{0, 8}`: two 8-byte words at offsets 0 and 8). No change needed; the
existing lowering is already correct for the shape the new allocator
produces.

## 5. Soundness argument (why `register_kind`/boxed mode is fine here,
unlike `vm-core`'s tagged-word case `AOT00-T10` had to invent a new mode for)

`AOT00-T10`'s tagged mode exists because `vm-core`'s NaN-boxed fields can
legitimately hold either a reference or a raw scalar in the *same*
dynamically-typed slot, making boxed mode's "every slot is always a
reference candidate" assumption unsound. LLVM's record/union constructor
cell is different: `twig-ir-compiler::emit_record_def` types every
constructor parameter `any`, and `iir-builtin-lowering::dyn_repr`'s
NaN-boxing convention means an `any`-typed word is *always* a tagged
`DynValue` (a real reference, a shifted int, or another lisp-tagged
scalar) — never a raw, untagged machine word. `register_kind`'s boxed mode
already handles this correctly today (it's how native's identical `{0,8}`
kind has worked since `AOT00-T3`): `mark_word`/`classify_precise_word`/
`forwarded` try both the raw and NaN-box-tag-stripped interpretations of
each field word, so a genuine tagged-int or tagged-reference field is
always resolved correctly, at the cost of the same astronomically-unlikely
look-alike-collision risk every other boxed-mode kind in this codebase
already accepts (see `AOT00-T10`'s spec §1 for why that same risk is
*not* acceptable for `vm-core`'s raw-untagged-word case — the two
situations are genuinely different, not the same gap reappearing).

## 6. What's still out of scope

- **WASM's linear-memory strings/arrays** (`AOT00-T1x` stage 2) — unrelated
  backend, unrelated gap, not touched here.
- **The `array<str>`/`array<any>`/`array<symbol>` ref-array gap** —
  `AOT00-T1w` §5's own flagged cross-backend follow-up (`alloc_array`, not
  `alloc`); this spec's `alloc` fix doesn't touch it.
- **A second movable kind for any non-pair Twig shape** — Twig's frontend
  only ever emits the 2-word pair shape for `alloc` (confirmed:
  `emit_record_def` always builds records as a cons chain of pairs, exactly
  like native's own constraint); nothing here needs a general N-field
  registration story.

## 7. Tests

- `iir-to-llvm/tests/test_backend.rs`: unit tests over the emitted IR
  string — `alloc_default_size_emits_gc_alloc_pair_and_declare` (no `srcs`
  operand → `@__twig_gc_alloc_pair()`, `declare i64 @__twig_gc_alloc_pair()`
  present, `@__twig_gc_alloc(` absent), `alloc_explicit_16_emits_gc_alloc_pair`
  (explicit `Operand::Int(16)` takes the same branch as no-operand),
  `alloc_non_pair_size_emits_conservative_gc_alloc` (e.g. explicit `32` still
  emits `@__twig_gc_alloc(i64 32)`, unchanged from today), `field_store_
  field_load_unchanged_for_pair_alloc` (regression: the existing raw-word
  field ops still lower identically regardless of which allocator produced
  the object).
- `lang-aot/tests/lang_matrix.rs` (or a new focused test file mirroring
  `llvm_gc_completion.rs`'s real-clang-execution pattern): a real,
  clang-compiled differential proving a record allocated on LLVM actually
  survives a real compacting/minor-compacting collection and relocates —
  the LLVM-specific analogue of this session's `vm-core` relocation proof
  (`gc_alloc_pair_relocates_and_stays_correct_under_a_compacting_minor_
  collection`), gated behind `clang_available()` like every other
  Twig-on-LLVM test in that file.
- Re-run the full existing `lang_matrix.rs` Llvm-column Twig cases
  (`record_first_field_runs_on_llvm`, `record_second_field_runs_on_llvm`,
  `union_match_runs_on_llvm`, `closures_run_on_llvm`) to confirm no
  regression — these must still pass byte-for-byte, since the allocator
  swap doesn't change any field's value or layout, only its GC kind.
