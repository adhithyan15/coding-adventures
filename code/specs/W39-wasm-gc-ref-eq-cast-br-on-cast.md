# W39 — WASM GC `ref.eq` / `ref.test`+`ref.cast` extension / `br_on_cast`+`br_on_cast_fail` / `any.convert_extern`+`extern.convert_any`

## Purpose and how this slice was chosen

After W38 (`code/specs/W38-wasm-gc-array-bulk-ops.md`) closed the array
bulk-ops cluster, the largest remaining coherent-looking cluster in the
257-file pinned testsuite (`28864811cf03bdbf880733786148feaba339582d`) was
reported as six files sharing the `ref.eq`/`ref.test`/`ref.cast`/
`br_on_cast`/`br_on_cast_fail`/i31 family: `ref_eq.wast` (83 NYS),
`ref_test.wast` (71), `i31.wast` (46), `ref_cast.wast` (45),
`br_on_cast_fail.wast` (31), `br_on_cast.wast` (31) — a nominal 307
directives. A prior session's own investigation had already flagged that
`ref.test`/`ref.cast` only support concrete **function** reference types,
not struct/array.

Per this campaign's own standing discipline — every W32-W38 spec
re-verified its own motivating claim directly against the pinned corpus
and current source, and every one of them found something the motivating
document got wrong, stale, or under-specified — this spec re-derives the
cluster's real current scope from scratch: current-`main`
(`24f24d2ea055db67119295cea4c1c123fb993e83`) `cargo run --release --bin
wasm_conformance_report` for the exact per-file tallies, then a throwaway
probe (`wasm_conformance::run_wast_source`, printing every distinct
`NotYetSupported` message with the directive ordinal and, by grepping the
fixture at the reported byte offset, the exact source text that triggered
it) for each of the six files.

**The re-verification confirms the six numbers above are still exactly
current — nothing has shifted since the milestone report — but finds the
307 is not one cluster. `i31.wast` is barely related to the other five at
all: only 4 of its 46 NYS trace to this spec's own target instructions;
the other 42 are two separate, unrelated, pre-existing gaps.** The other
five files' 261 NYS all genuinely trace to this spec's five target
instructions/instruction-pairs, split across four distinct causes. See
"Correction 1" and "Correction 2" below.

## Correction 1: `i31.wast`'s 46 NYS are 91% a DIFFERENT, unrelated cluster — only 4 belong here

Byte-position-verified breakdown of `i31.wast`'s 46 `NotYetSupported`
directives (module/action/assert_return kinds all included):

| Cause | Count | Byte offset(s) | Source text |
|---|---:|---|---|
| Flat-form `table.size $table` / `table.grow $table` — a **named table operand on a non-folded, one-atom-per-line instruction**, not a GC gap at all | 38 | 2646, 6315 | `table.size $table` (appears standalone on its own line, not `(table.size $table)`) |
| `(table $t 3 3 (ref i31) (ref.i31 (global.get $g)))` — an **inline table-initializer expression** for a non-funcref/externref reftype table declaration (the function-references proposal's `limits reftype init_expr` table form) | 4 | 4844 | `(table $t 3 3 (ref i31) (ref.i31 (global.get $g)))` |
| `(ref.cast i31ref (global.get $c))` — a **bare abstract-reftype-keyword** (`i31ref`) used as `ref.cast`'s own type immediate | 4 | 5616 | `(i31.get_u (ref.cast i31ref (global.get $c)))` |
| **Total** | **46** | | |

The first cause (`table.size`/`table.grow` with a **bare identifier
operand in flat/non-folded instruction form**) is a general multi-table
parsing gap with zero connection to GC: `wasm-wast-parser`'s flat-form
instruction stream evidently doesn't yet consume a trailing `$id`/index
operand for `table.size`/`table.grow` the way it does for the equivalent
folded form `(table.size $table)`, so the parser treats `$table` as if it
started a brand-new instruction and rejects it as unknown. This belongs
with whatever future spec covers multi-table flat-instruction-stream
support, not this one.

The second cause (inline table-initializer expressions) is the SAME gap
this repo's own `wasm-wast-parser::module::build_table_limits_and_
elements` already self-diagnoses in its own doc comments (referenced,
unprompted, by the probe's error message itself: "the function-references
proposal's `limits reftype init_expr` table form is not yet supported for
a non-funcref/externref reftype"). It is a real, separate, already-known
gap, orthogonal to `ref.test`/`ref.cast`/`ref.eq`/`br_on_cast`.

Only the third cause — `ref.cast`/`ref.test` given a **bare abstract
reftype keyword** as their own type immediate — is this spec's business,
and it's the SAME underlying parser limitation Correction 2 below covers
for the other five files (so no new work is needed for `i31.wast`
specifically; fixing Correction 2's gap for `ref_test.wast`/`ref_cast.
wast` automatically fixes these 4 `i31.wast` directives too).

**Net: this spec's real, addressable scope is 83 + 71 + 45 + 31 + 31 + 4 =
265 of the reported 307 directives.** The other 42 (all in `i31.wast`) are
explicitly out of scope — see "Explicitly out of scope" below.

## Correction 2: `ref.test`/`ref.cast`'s "abstract heap type" rejection is two different gaps wearing one error message

`wasm-wast-parser/src/module.rs`'s `ref.test`/`ref.cast` encoder (~line
4903) parses its type immediate with `parse_value_type`, then accepts only
two of that function's ~20 possible return variants:

```rust
let (type_idx, nullable) = match parse_value_type(ty_expr, &icx.module.type_names, &icx.module.module)? {
    ValueType::NonNullConcreteFuncRef(idx) => (idx, false),
    ValueType::ConcreteFuncRef(idx) => (idx, true),
    _ => {
        return Err(WastParseError::UnexpectedToken {
            pos: ty_expr.pos(),
            found: "abstract heap type".to_string(),
            expected: "a concrete (ref $t) / (ref null $t) heap type",
        });
    }
};
```

Every other `ValueType` `parse_value_type` can return — including a
**genuinely concrete, non-abstract** struct/array reference produced by
`concrete_ref_value_type` when `$t` names a struct or array type — falls
into the same `_` arm and is reported with the same, **mislabeled**
"abstract heap type" message. Two real corpus cases hit this one arm for
two different reasons:

1. **`ref_test.wast` byte 8971 / `ref_cast.wast` byte 5266**: `(ref.test
   (ref null $t0) (ref.null struct))` where `$t0` is declared `(type $t0
   (struct))`. `parse_value_type` correctly resolves `(ref null $t0)` to
   `ValueType::StructRef($t0's index)` via `concrete_ref_value_type` — a
   real, CONCRETE, non-abstract heap type — but the match arm above only
   recognizes the two `ConcreteFuncRef`/`NonNullConcreteFuncRef` variants,
   so a legitimate concrete **struct** type falls through and is rejected
   as if it were abstract. This is the prior session's already-flagged
   "only concrete function types" gap, confirmed live and pinned to this
   exact code site.

2. **`i31.wast` byte 5616 / `ref_cast.wast` byte 449 region / `ref_test.
   wast`'s own use elsewhere**: `(ref.cast i31ref ...)` — a **bare
   abstract-hierarchy keyword** (`i31ref`, and by the same code path
   `eqref`/`structref`/`arrayref`/`anyref`/`funcref`/`externref`) used
   directly as `ref.test`/`ref.cast`'s type immediate. `parse_value_type`
   correctly resolves `i31ref` to `ValueType::I31ref` — a real, GENUINELY
   ABSTRACT heap type this time — which also falls into the same `_` arm.
   This is a real, still-unimplemented case: testing/casting against an
   abstract hierarchy tier (not a concrete type) needs different runtime
   semantics (structural "which kind of value is this", not nominal
   subtype-chain lookup — see Design §2 below).

**These are two different fixes bundled behind one rejection branch**:
(1) is "recognize the same concrete-struct/array `ValueType`s
`concrete_ref_value_type` already produces, encode+dispatch them like the
existing funcref path"; (2) is "add a genuinely new runtime code path for
abstract-heap-type dynamic tests, which the funcref-only stub was never
designed to answer." Both must ship for `ref_test.wast`/`ref_cast.wast` to
pass; only (2) matters for `i31.wast`'s 4 directives.

## Correction 3: `br_on_cast`/`br_on_cast_fail`'s own corpus needs a `ValueType` this repo doesn't have yet: non-null `anyref`

Both `br_on_cast.wast` and `br_on_cast_fail.wast` declare function
signatures like:

```wat
(func (param (ref any)) (result (ref any))
  (block (result (ref $t)) (br_on_cast_fail 1 (ref any) (ref $t) (...))))
```

`(ref any)` is the **non-null** abstract top reference type. Reading
`wasm-wast-parser::module::parse_value_type`'s non-null `(ref X)` compound
branch (module.rs:390-397) directly shows it special-cases only `i31`
(→ `ValueType::I31ref`, no non-null/nullable distinction modeled) and
`array` (→ `ValueType::NonNullArrayAny`, added in W38); every other atom —
`any`, `eq`, `struct`, `func`, `extern` — falls through to `resolve_idx`
and fails with "expected an index, found \"any\"" (confirmed live at
`br_on_cast.wast` byte 8832 and `br_on_cast_fail.wast` byte 10121). Unlike
Correction 2's cases, this is not just a missing parser branch: `wasm-
types::ValueType` (`code/packages/rust/wasm-types/src/lib.rs`) has real
`NonNullStructRef(u32)`/`NonNullArrayRef(u32)`/`NonNullConcreteFuncRef(u32)`
/`NonNullArrayAny` variants but **no non-null abstract `any`/`eq`/`struct`
variant at all** — the same kind of net-new-variant addition W38 already
did once for `NonNullArrayAny`, needed again here one level up the
hierarchy.

## Real spec text, fetched directly and quoted/paraphrased faithfully

### Binary opcodes

Fetched from `https://raw.githubusercontent.com/WebAssembly/gc/main/
proposals/gc/MVP.md`'s own binary-format tables, then cross-checked
against this repo's own already-shipped, corpus-verified `0x00`-`0x1E`
assignments (`wasm-module-encoder/src/lib.rs`'s `GcInstruction` doc
comments, and `wasm-execution`/`wasm-validator`'s matching `0xFB` match
arms) — every existing byte this repo already uses (`0x14`-`0x17` for
`ref.test`/`ref.cast`, `0x1C`-`0x1E` for `ref.i31`/`i31.get_s`/
`i31.get_u`) matches the spec verbatim, so the gaps in the same numbering
sequence are trustworthy:

| Instruction | Opcode | Immediates | Status in this repo |
|---|---|---|---|
| `ref.eq` | **`0xD3`** (single byte, base opcode space — NOT under the `0xFB` prefix at all) | none | **Unimplemented anywhere** (confirmed: no match arm in `wasm-execution`, `wasm-validator`, or `wasm-wast-parser`; `0xD3` is free — `0xD0`=`ref.null`, `0xD1`=`ref.is_null`, `0xD2`=`ref.func` are the only base-space reference opcodes this repo uses) |
| `ref.test (ref ht)` | `0xFB 0x14` | `ht: heaptype` | Implemented, funcref-only (Correction 2) |
| `ref.test (ref null ht)` | `0xFB 0x15` | `ht: heaptype` | Implemented, funcref-only |
| `ref.cast (ref ht)` | `0xFB 0x16` | `ht: heaptype` | Implemented, funcref-only |
| `ref.cast (ref null ht)` | `0xFB 0x17` | `ht: heaptype` | Implemented, funcref-only |
| `br_on_cast` | `0xFB 0x18` | `flags: u8, $l: labelidx, ht1: heaptype, ht2: heaptype` | **Unimplemented anywhere** |
| `br_on_cast_fail` | `0xFB 0x19` | `flags: u8, $l: labelidx, ht1: heaptype, ht2: heaptype` | **Unimplemented anywhere** |
| `any.convert_extern` | `0xFB 0x1A` | none | Enum variant exists in `wasm-module-encoder` only (`GcInstruction::AnyConvertExtern`, doc-commented "convert an externref to anyref"); **not wired into the text parser, validator, or executor** |
| `extern.convert_any` | `0xFB 0x1B` | none | **Unimplemented anywhere** (not even an encoder stub) |
| `ref.i31` | `0xFB 0x1C` | none | Implemented (W20) |
| `i31.get_s` | `0xFB 0x1D` | none | Implemented (W20) |
| `i31.get_u` | `0xFB 0x1E` | none | Implemented (W20) |

`br_on_cast`/`br_on_cast_fail`'s flags byte: bit 0 = "first reftype is
nullable" (`null1?`), bit 1 = "second reftype is nullable" (`null2?`),
per the same source document.

A second, independent fetch of `https://webassembly.github.io/gc/core/
binary/instructions.html` via a summarizing fetch tool initially returned
**wrong** sub-opcode numbers for the `0xFB` range (`0x20`-`0x27` instead
of `0x14`-`0x1B`) while getting `ref.eq`'s `0xD3` right — a real
extraction error in that one fetch, caught only by cross-checking against
this repo's own already-tested, already-passing `0x14`-`0x1E` assignments
and a second, independent fetch of the MVP.md source doc, which agreed
with the repo and with `ref.eq = 0xD3` from the first fetch. **Numbers in
the table above are the ones that survived cross-checking; a future
implementer should not trust a single fetch of this specific page
without the same cross-check.**

### Validation rules (`https://webassembly.github.io/gc/core/valid/instructions.html`, quoted)

- **`ref.eq`**: "valid with type `[(ref null eq) (ref null eq)] → [i32]`."
  Both operands are popped as `eqref` (nullable top of the `eq`
  hierarchy — covers `i31ref`, `structref`, `arrayref`, and their
  concrete subtypes, plus null); result is `i32` (1 if referentially
  equal, including null == null, else 0).
- **`ref.test rt`**: "valid with type `[rt'] → [i32]` for any valid
  reference type `rt'` for which `rt` matches `rt'`" — i.e. the OPERAND's
  static type `rt'` must be a type `rt` could plausibly narrow (any
  common ancestor relationship is enough for validity; the actual
  test is a RUNTIME check).
- **`ref.cast rt`**: "valid with type `[rt'] → [rt]`" — same operand
  constraint as `ref.test`, but the pushed type is the concrete `rt`
  itself (a successful cast statically narrows the value); a failed cast
  traps at runtime.
- **`any.convert_extern`**: "valid with type `[(ref null₁? extern)] →
  [(ref null₂? any)]` for any `null₁?` that equals `null₂?`" — an
  identity-preserving bridge that carries nullability through unchanged.
- **`extern.convert_any`**: the mirror image, `[(ref null₁? any)] →
  [(ref null₂? extern)]`, same nullability-preservation rule.

### `br_on_cast`/`br_on_cast_fail`'s formal typing rule (fetched from `MVP.md`, quoted verbatim)

```
br_on_cast $l rt1 rt2 : [t0* rt1] -> [t0* rt1\rt2]
  iff $l : [t0* rt2]
  and rt2 <: rt1
  (branches with the operand retyped as rt2 if it matches; otherwise
   falls through with the operand retyped as rt1\rt2)
  if rt2 contains null, branches on null; otherwise does not

br_on_cast_fail $l rt1 rt2 : [t0* rt1] -> [t0* rt2]
  iff $l : [t0* rt1\rt2]
  and rt2 <: rt1
  (branches with the operand retyped as rt1\rt2 if it does NOT match;
   otherwise falls through with the operand retyped as rt2)
  if rt2 contains null, does NOT branch on null; otherwise does

where the type-difference operator \ is defined:
  (ref null1? ht1) \ (ref null ht2)  = (ref ht1)          -- rt2 nullable: fallthrough/fail-branch value is proven non-null
  (ref null1? ht1) \ (ref ht2)       = (ref null1? ht1)   -- rt2 non-null: nullability of rt1 is unaffected/unproven
```

In plain terms: `rt1`/`rt2` are both given directly as the instruction's
own two heap-type immediates (never inferred from stack contents), `rt2`
must be a real subtype of `rt1` (checked once, statically, at validation
time), the branch target's OWN declared label types must already expect
whichever narrowed type that instruction produces on the taken path, and
whether a **null** value takes the branch or falls through depends purely
on `rt2`'s own nullability bit (not `rt1`'s) — this is the one piece of
real, RUNTIME-observable behavior in this pair of instructions that must
be implemented exactly right; the *static* type-narrowing is (per Design
§4 below) largely a bookkeeping exercise this specific codebase's own
looser validator already mostly sidesteps.

## Current implementation, read directly

### `wasm-opcodes/src/lib.rs`

No entries for any `0xFB`-prefixed instruction at all (confirmed:
`grep -n "RefTest\|RefCast\|BrOnCast\|RefEq" wasm-opcodes/src/lib.rs`
returns nothing). This crate's `OpcodeInfo` table is for the flat,
single-byte-opcode MVP-era instruction space; every `0xFB` GC instruction
is dispatched via its own hand-rolled sub-opcode `match` registered
through `register_context_opcode(0xFB, ...)` in `wasm-execution` and a
parallel `match` in `wasm-validator`, bypassing this table entirely — the
existing, established convention (W37/W38 made no changes here either).
**No changes needed in this crate for this spec.**

### `wasm-execution/src/lib.rs`

- The `0xFB` dispatch closure (registered via `vm.register_context_
  opcode(0xFB, ...)`) currently handles `0x00`-`0x13` (struct/array bulk
  ops) and `0x14`/`0x15`/`0x16`/`0x17` (ref.test/ref.test null/ref.cast/
  ref.cast null) and `0x1C`-`0x1E` (i31), with an `other => Err(...
  "unsupported WasmGC opcode 0xFB 0x{other:02X}")` catch-all — so `0x18`/
  `0x19`/`0x1A`/`0x1B` currently trap cleanly with a real error, not a
  silent misbehavior, once the parser is extended to emit them.
- `ref_matches_concrete_type` (line ~5636) is the dynamic-check helper
  `ref.test`/`ref.cast` both call. It disambiguates "is `type_idx` a
  function type or a struct/array type" purely via `type_idx <
  ctx.types.len()` (this repo's documented convention: "struct type k is
  at type-section index `types.len() + k`"). The func-typed branch does a
  REAL nominal check: resolve the value's actual function-type index
  (through `func_type_indices`, handling both a raw local index and a
  tagged `func_ref_heap` handle per W35), then
  `wasm_types::nominal_subtype_chain(&ctx.type_subtyping, &ctx.canonical_types, actual_type_idx, type_idx)`.
  The struct/array-typed branch is a **documented stub**:
  `ctx.gc_heap.get(payload as usize).and_then(|slot| slot.as_ref()).is_some()`
  — "any live struct-heap object matches any concrete struct type,"
  correct only because this engine, pre-W39, only ever allocates one
  struct shape (`$LispyPair`, for McCarthy pairs).
- `GcObject` (line ~4189) has exactly two variants, `Struct(GcStruct)`/
  `Array(GcArray)`, and **both already carry their own concrete `type_idx`
  field** (`GcStruct { type_idx: u32, fields }`, `GcArray { type_idx: u32,
  elements }`, populated by `struct.new`/`array.new`'s own handlers) —
  this is the exact piece of information `ref_matches_concrete_type`'s
  struct/array stub needs but never reads, and it is already sitting
  right there on every heap object. Extending the stub to a real nominal
  check is `match gc_object { Struct(s) => s.type_idx, Array(a) =>
  a.type_idx }` then the SAME `nominal_subtype_chain` call the funcref
  path already makes — no new subtyping machinery required (Design §2).
- `ref.null`'s own runtime handler (`0xD0`, line ~7335) is a one-liner
  that always pushes `WasmValue::Ref(None)` — it never even reads the
  heap-type immediate byte at runtime, because a null value's identity
  doesn't depend on its declared type in this engine's model. This means
  there is **no existing generic "decode a heap-type immediate" runtime
  helper** to reuse for `ref.test`/`ref.cast`/`br_on_cast`'s own
  immediates; one must be written fresh (Design §2/§4).
- `i31.wast`'s own `i31.get_s`/`i31.get_u` handlers (`pop_i31_payload`,
  line ~5691) confirm `i31ref` values are carried as a plain
  `WasmValue::I32` on the operand stack, NOT as a `WasmValue::Ref` at
  all — a real, load-bearing detail for the abstract-heap-type dynamic
  test (Design §2's open question).
- `GcObject` has no third variant for externref — confirmed by reading
  the enum directly (only `Struct`/`Array`). A doc comment elsewhere
  ("externref table entry is a `ctx.gc_heap` handle, not a function...")
  suggests externref values are stored via the same heap in some table
  contexts, but this spec did **not** fully chase down how an externref
  value is represented well enough to write a definitive "which universe
  does this value belong to" rule for `any.convert_extern`/`extern.
  convert_any` and for testing `anyref`/`eqref` against a possibly-extern-
  sourced value — flagged as an explicit open sub-question for whichever
  slice implements it (Design §2/§3).

### `wasm-validator/src/type_check.rs`

- The `0xFB` match arm (line ~2241) mirrors `wasm-execution`'s dispatch:
  `0x14`/`0x15` pop one value (any type — `pop_val` takes no expected-type
  argument, so this validator does not even check the popped value looks
  reference-shaped) and push `ValueType::I32`; `0x16`/`0x17` pop one value
  and push `StackType::Unknown` (never a precise narrowed type); `0x1C`-
  `0x1E` (i31) similarly pop/push generically. Both `0x14`-`0x17` arms
  DO correctly consume their heap-type LEB128 immediate bytes (needed
  just to keep `offset` in sync with the real instruction stream), but
  never inspect the decoded value for anything beyond its byte length.
- **This validator's `StackType` enum has a permissive `Unknown` variant
  that satisfies ANY expected type check** (`pop_val`/`pop_expect`'s
  `Unknown => Ok(())` arms, confirmed at three call sites). This is the
  single most important design fact for `br_on_cast`/`br_on_cast_fail`'s
  own validation (Design §4): this codebase's validator was never built
  to track precise reference subtypes through the abstract interpreter in
  the first place (`ref.cast`'s own existing arm already pushes `Unknown`
  unconditionally, not the real narrowed `rt`), so the "hard" part of the
  real spec's typing rule — computing and threading the exact `rt1\rt2`
  difference type through the operand stack — can be sidestepped the same
  way `ref.cast` already sidesteps it, **without** creating a real
  regression relative to this validator's own existing rigor level
  elsewhere in the same instruction family.
- `resolve_label_target`/`label_types`/`pop_expect_many`/`push_vals` (the
  exact helpers `br`/`br_if` at `0x0C`/`0x0D` already use, lines
  2956-2976) are the correct, reusable machinery for `br_on_cast`/
  `br_on_cast_fail`'s own branch-target validation — same shape as
  `br_if`, just with the "condition" being the dynamic cast test instead
  of a popped `i32`.
- `ref.null`'s own `0xD0` arm (line 2654) decodes a heap-type byte with an
  explicit `match` over the small fixed set of known tag bytes (`0x70`
  func, `0x6F` extern, `0x73`/`0x72`/`0x74`/`0x71` the four bottom types,
  `0x63` a repo-internal "concrete index follows" tag it invented rather
  than matching the real spec's own sign-disambiguated encoding) — but
  notably does **not** have arms for `0x6E` (any)/`0x6D` (eq)/`0x6B`
  (struct)/`0x6C` (i31)/`0x69` (exn), all of which fall through to the
  generic `_ => StackType::Unknown`. This confirms `ref.null`'s own
  "`0x63` + LEB index" convention is a deliberate, documented, **non-spec**
  simplification specific to that one instruction, NOT the pattern
  `ref.test`/`ref.cast`/`br_on_cast` should copy — their own heap-type
  immediate (per Design §2/§4) should decode the REAL spec encoding
  directly (a signed LEB128 where small non-negative values are indices
  and the specific negative single-byte values are abstract tags),
  reusing the exact byte constants `wasm-wast-parser::parse_ref_null_
  heap_type` already established as correct (`0x6E` any, `0x6D` eq,
  `0x6B` struct, `0x6A` array, `0x6C` i31, `0x70` func, `0x6F` extern,
  `0x71`/`0x73`/`0x72`/`0x74` the four bottom types, `0x69` exn) — because
  `ref.test`/`ref.cast`'s CURRENT encoder already emits a bare LEB128
  type index with no `0x63` wrapper (`out.extend(wasm_leb128::encode_
  unsigned(type_idx as u64))`), which is the real spec's own convention,
  not `ref.null`'s repo-internal one.

### `wasm-wast-parser/src/module.rs`

- `parse_value_type` (line 310): the 3-item `(ref null X)` branch already
  recognizes `func`/`extern`/`i31`/`eq`/`struct` as abstract atoms (W08/
  W20/W37) and falls back to `concrete_ref_value_type` for anything else
  (any named/numeric type, dispatched to func/struct/array by `module.
  type_kinds`, W33 fourth slice) — already fully general for the
  NULLABLE compound form. The 2-item **non-null** `(ref X)` branch (line
  395) only special-cases `i31` and `array`, then falls through to
  `resolve_idx` for everything else — `any`/`eq`/`struct`/`func`/`extern`
  are NOT recognized as non-null abstract atoms here at all (Correction
  3). The bare-atom match (line 405) already has `eqref`/`structref`/
  `arrayref`/`anyref`/`i31ref`/`funcref`/`externref` — fully general.
- `ref.test`/`ref.cast`'s own encoder (line 4903, quoted in Correction 2)
  is the single site needing extension for both of Correction 2's cases.
- `parse_ref_null_heap_type` (line 478) already has the exact correct
  byte-tag table this spec's runtime/validator decoders should reuse.
- **No existing GC-related opcode encoder writes `br_on_cast`/`br_on_
  cast_fail`/`ref.eq`/`any.convert_extern`/`extern.convert_any` at all** —
  confirmed by `grep`, zero hits for any of the five instruction names in
  this file outside comments describing the corpus.

### `wasm-types/src/lib.rs`

- `ValueType` (line 125) has `NonNullStructRef(u32)`/`NonNullArrayRef(u32)`
  /`NonNullConcreteFuncRef(u32)`/`NonNullArrayAny` but no non-null
  abstract `any`/`eq`/`struct` variant (Correction 3).
- `nominal_subtype_chain` (line 1904) is already generic over type
  indices (not funcref-specific) — directly reusable for the struct/array
  extension in Design §2, no changes needed to this function itself.
- `any_declares_subtyping` (line 2061) gates the entire dynamic-check path
  (funcref included) on whether the module declares any real subtyping at
  all; this gate is reused unchanged.

## Design

### Slice 1 — `ref.eq` (simplest, fully isolated; no dependency on anything else here)

- **`wasm-wast-parser`**: add `ref.eq` to the flat-instruction-name match
  (same shape as `ref.is_null`/`ref.i31`: no immediate, folded operands
  only, `encode_instr_list(args, icx, out)?; out.push(0xD3);` — single
  base-opcode byte, no `0xFB` prefix, matching the fetched spec exactly).
- **`wasm-execution`**: register a plain (non-context, or context if
  needed for consistency) `0xD3` handler: pop two values, push `I32(1)`
  if referentially equal else `I32(0)`. Per the real spec's `ref.eq`
  semantics: two **null** values are equal; two `i31ref` values (plain
  `I32` payloads in this engine) are equal iff their unboxed integers are
  equal; two `gc_heap`-handle values are equal iff they're the SAME handle
  (reference identity, not structural/deep equality — this repo's `Vec`-
  index handle scheme already gives this for free, no new machinery).
  Cross-kind comparisons (e.g. an i31 payload vs. a struct handle) are
  always `0`, never a trap (validation, not execution, is where a genuine
  type mismatch would be caught in a fully-typed engine — this repo's own
  loose-validator convention makes a defensive runtime `false` the right
  choice here, matching `ref_matches_concrete_type`'s own `_ => false`
  discipline for a numeric/non-ref payload).
- **`wasm-validator`**: add a `0xD3` arm alongside the existing `0xD0`-
  `0xD2` ones: pop two values (generic `pop_val`, matching this crate's
  existing looseness for every other GC instruction), push `I32`.

### Slice 2 — `ref.test`/`ref.cast` extension (both Correction-2 cases; unblocks `ref_test.wast`, `ref_cast.wast`, and `i31.wast`'s 4 in-scope directives)

**2a. Concrete non-func heap types** (the nominal case, e.g. `(ref $t0)`
where `$t0` is a struct/array type):

- `wasm-wast-parser`: extend the `ref.test`/`ref.cast` encoder's match to
  accept `ValueType::NonNullStructRef`/`StructRef`/`NonNullArrayRef`/
  `ArrayRef` alongside the existing two funcref variants, encoding the
  SAME `type_idx`/nullable pair (the struct/array index space is already
  `types.len() + k`, so the existing `wasm_leb128::encode_unsigned(type_idx
  as u64)` needs no format change — same bytes, richer source variants).
- `wasm-execution`: extend `ref_matches_concrete_type`'s struct/array
  branch (currently `.is_some()`) to a real nominal check:
  ```rust
  match ctx.gc_heap.get(payload as usize).and_then(|slot| slot.as_ref()) {
      Some(GcObject::Struct(s)) => wasm_types::nominal_subtype_chain(&ctx.type_subtyping, &ctx.canonical_types, s.type_idx, type_idx),
      Some(GcObject::Array(a)) => wasm_types::nominal_subtype_chain(&ctx.type_subtyping, &ctx.canonical_types, a.type_idx, type_idx),
      None => false,
  }
  ```
  — reusing the exact function the funcref path already calls; no new
  subtyping algorithm.
- `wasm-validator`: no change needed beyond what already exists (the
  `0x14`-`0x17` arms already accept any heap-type-immediate byte length
  generically).

**2b. Genuinely abstract heap types** (e.g. bare `i31ref`, `eqref`,
`structref`, `arrayref`, `anyref`, `funcref`, `externref` as the type
immediate):

- `wasm-wast-parser`: extend the same encoder match to accept
  `ValueType::I31ref`/`Eqref`/`StructRefAny`/`ArrayRefAny`/`Anyref`/
  `Funcref`/`Externref` (and their `NonNull*` counterparts once Correction
  3 adds them), encoding the heap-type immediate as the SAME single
  tag byte `parse_ref_null_heap_type` already establishes as correct
  (`0x6C` i31, `0x6D` eq, `0x6B` struct, `0x6A` array, `0x6E` any, `0x70`
  func, `0x6F` extern) instead of a LEB128 type index — this is a real
  fork in the encoder (a small negative-tag byte vs. a positive LEB128
  index), not a one-line addition.
- `wasm-execution`/`wasm-validator`: both need a NEW decode step before
  calling into `ref_matches_concrete_type` — peek the immediate's first
  byte; if it matches one of the known abstract tag bytes, dispatch to a
  new **structural** test function instead of the nominal one. The
  structural test's rule, per the real GC type hierarchy: `funcref`
  matches a func-shaped value or null (if nullable); `externref` matches
  an extern-shaped value or null; `i31ref` matches a plain `WasmValue::
  I32` payload; `structref`/`arrayref` match a `GcObject::Struct`/`Array`
  heap value; `eqref` matches any of i31/struct/array; `anyref` matches
  any of i31/struct/array (but NOT func/extern — those are genuinely
  separate hierarchies bridged only by `any.convert_extern`/`extern.
  convert_any`, never implicitly). **Open sub-question, explicitly
  flagged rather than resolved here**: this engine's `GcObject` enum has
  no distinct externref-carrying variant, and this spec did not fully
  trace how an externref value already flowing through the engine (via
  table entries, `ref.extern`, or host imports) is represented at
  runtime — whoever implements this slice should read that path fresh
  before writing the `externref`/`anyref` disambiguation arm, rather than
  assuming the funcref-vs-struct/array split (`type_idx < ctx.types.len()`)
  extends cleanly to a third case.

### Slice 3 — `any.convert_extern`/`extern.convert_any` (small, contained; independent of slices 1/2/4)

- `wasm-wast-parser`: add both names to the flat-instruction match
  (`encode_instr_list(args, icx, out)?; out.push(0xFB); out.push(0x1A /* or 0x1B */);`
  — no immediate, matching `ref.i31`'s own shape exactly). Note `wasm-
  module-encoder` already has a `GcInstruction::AnyConvertExtern` variant
  emitting `0xFB 0x1A` (used elsewhere, e.g. by a different language
  backend that targets this repo's own encoder) — the text-parser side
  needs the SAME byte, so both stay consistent.
- `wasm-execution`: add `0x1A`/`0x1B` arms to the existing `0xFB` dispatch
  closure. Per the fetched validation rule, these are **identity-
  preserving** — pop one `WasmValue::Ref`, push it back completely
  unchanged (this repo's `WasmValue::Ref(Option<u32>)` representation
  doesn't distinguish "extern-flavored" from "any-flavored" at the value
  level at all — the conversion is purely a STATIC/type-system fiction in
  this engine's own model, same conclusion `ref.null`'s doc comments
  already draw for other abstract-hierarchy bridges). No trap conditions.
- `wasm-validator`: add `0x1A`/`0x1B` arms: pop one value, push
  `StackType::Unknown` (or, if slice 2b's abstract-type infrastructure
  already computed real `Anyref`/`Externref` `ValueType`s by this point,
  push the precise pushed type — a nice-to-have, not required for the
  corpus to pass, since `Unknown` already satisfies any downstream
  `is_assignable` check per this validator's existing convention).

### Slice 4 — `br_on_cast`/`br_on_cast_fail` (hardest; depends on slice 2's heap-type-immediate decoder existing first, and slice 3's `NonNullAnyref`-family variant from Correction 3)

**Prerequisite (Correction 3)**: add `ValueType::NonNullAnyref`/
`NonNullEqref`/`NonNullStructRefAny` (whichever the real corpus actually
needs — direct read shows only `(ref any)` is used by the six-file
cluster, so `NonNullAnyref` alone may suffice; check `type-subtyping.wast`
and any other corpus user of `(ref eq)`/`(ref struct)` non-null forms
before deciding whether to add all three or just the one proven-needed
variant, following this campaign's own "don't build untested surface"
discipline). Wire it into `parse_value_type`'s non-null `(ref X)` branch,
`ValueType::is_assignable`/subtyping lattice, and `byte_tag()`/`encode()`,
mirroring exactly how W38 added `NonNullArrayAny`.

- `wasm-wast-parser`: add `br_on_cast`/`br_on_cast_fail` to the
  instruction encoder. Text-format operands per the real grammar:
  `br_on_cast $l (ref null1? ht1) (ref null2? ht2) <value>` — parse `$l`
  via the existing label-resolution helper (same as `br`/`br_if`'s own
  encoder), parse both reftypes via `parse_value_type` (now fully general
  after slice 2), derive the flags byte from each reftype's own
  nullability, and emit `0xFB 0x18/0x19 <flags:u8> <labelidx:LEB>
  <ht1-immediate> <ht2-immediate>` — reusing slice 2's own heap-type-
  immediate encoder verbatim for `ht1`/`ht2` (this is exactly why slice 2
  must land first: `br_on_cast`'s own two type immediates are encoded
  IDENTICALLY to `ref.test`/`ref.cast`'s single one, abstract-tag-byte or
  LEB128-index either way).
- `wasm-execution`: add `0x18`/`0x19` arms. Decode flags/labelidx/ht1/ht2,
  pop the operand, run the SAME dynamic test slice 2 built (nominal for a
  concrete `ht2`, structural for an abstract one) to decide match/no-
  match, then branch or fall through per the fetched rule's exact null-
  handling clause ("if rt2 contains null, branches on null" for
  `br_on_cast`; "does NOT branch on null" for `br_on_cast_fail" — this is
  the one piece of genuinely new, easy-to-get-backwards runtime logic;
  write a direct unit test asserting BOTH directions against a literal
  null operand before trusting it against the corpus). The actual branch
  mechanics (jumping to a target block, adjusting the operand stack by
  the label's arity) should reuse whatever internal helper `br`/`br_if`'s
  own `0x0C`/`0x0D` runtime handlers already use — read those first
  rather than reimplementing branch-taking from scratch.
- `wasm-validator`: add `0x18`/`0x19` arms modeled directly on `br_if`'s
  own (`0x0D`, line 2966-2975): decode flags/labelidx/ht1/ht2 (consuming
  the right number of immediate bytes is the only hard requirement — this
  validator's own `StackType::Unknown` convention, per the "Real spec
  text" section's closing paragraph, means the exact `rt1\rt2` difference
  type does NOT need to be computed and threaded through precisely to
  match this codebase's existing rigor level); pop one generic value
  (`pop_val`); resolve the target label via `resolve_label_target`/
  `label_types`, exactly like `br_if`; since this repo's `label_types`
  returns `ValueType`s and this instruction's own narrowed type isn't
  tracked precisely, push `StackType::Unknown` back for whichever path
  continues in-line (matching `ref.cast`'s own established precedent) —
  optionally validate `rt2 <: rt1` using `is_assignable` when both
  resolve to known, non-`Unknown` `ValueType`s, skipping the check
  permissively otherwise (same "don't reject what we can't yet precisely
  model" discipline `ref.null`'s `0xD0` arm already applies at its own
  `_ => Unknown` fallback).

### Slice 5 — full corpus re-verification

Re-run `cargo run --release --bin wasm_conformance_report` and the same
throwaway-probe method this spec itself used, confirming: `ref_eq.wast`,
`ref_test.wast`, `ref_cast.wast`, `br_on_cast.wast`, `br_on_cast_fail.
wast` all reach 100% Pass (0 NYS, 0 Fail); `i31.wast`'s NYS count drops
from 46 to 42 (only the two explicitly-out-of-scope causes remain); the
full 257-file aggregate's `assert_return`/`assert_invalid`/etc. NYS totals
drop by exactly 265 with zero new `Fail` outcomes anywhere (a regression
in, say, `type-subtyping.wast`'s existing funcref `ref.test`/`ref.cast`
cases, or `struct.wast`/`array.wast`'s own struct/array construction,
would show up here first).

## Trap conditions (security-relevant — real memory-safety/type-safety boundaries on this interpreter's own `gc_heap`, not merely spec conformance)

- `ref.cast` (existing behavior, unchanged): a failed cast traps
  ("cast failure"); slice 2 extends WHICH types can be tested, not
  whether a mismatch traps.
- `br_on_cast`/`br_on_cast_fail`: never trap on their own — a
  non-matching value simply takes the other control-flow path. The only
  new risk is a validator/executor **immediate-decoding** desync (reading
  the wrong number of bytes for `ht1`/`ht2` leaves every subsequent
  instruction in the function body misinterpreted) — the same class of
  bug this repo's own `array.init_elem` validator arm doc comment already
  calls out by name for a different instruction ("previously fell into
  the `_ => {}` no-immediate default... which would silently desync
  `offset`"). Slice 4's validator arm must consume exactly `1 (flags) +
  LEB(labelidx) + heap-type-immediate(ht1) + heap-type-immediate(ht2)`
  bytes, verified by a direct unit test with a KNOWN-length encoding, not
  just corpus-level pass/fail.
- `ref.eq`: no trap conditions at all per the real spec (even comparing
  two null references is well-defined, `i32` `1`).

## Explicitly out of scope for this spec

1. **`table.size $table`/`table.grow $table` in flat (non-folded)
   instruction-stream form** (`i31.wast`, 38 NYS) — a general multi-table
   text-format parsing gap, unrelated to GC. Belongs with a future
   multi-table/flat-instruction-stream spec.
2. **Inline table-initializer expressions for non-funcref/externref
   reftype table declarations** (`i31.wast`, 4 NYS) — the function-
   references proposal's own `limits reftype init_expr` table form,
   already self-flagged in this repo's existing `wasm-wast-parser` doc
   comments as a known, separate gap.
3. **A fully precise, spec-exact static type for `br_on_cast`/`br_on_
   cast_fail`'s pushed operand** (the real `rt1\rt2` computation) — Design
   §4 deliberately reuses this validator's existing `StackType::Unknown`
   convention instead. If a future spec ever upgrades this validator to
   track precise reference types generally (not just for this
   instruction pair), revisit this decision then; doing it only here,
   ahead of that broader upgrade, would be inconsistent with how `ref.
   cast` already behaves today.
4. **A structural test correctly distinguishing extern-sourced values
   from any-sourced ones at the abstract-heap-type level** beyond
   "flagged as an open sub-question" (Design §2b) — full resolution
   requires reading how externref values are constructed/stored
   end-to-end first, deliberately left to whoever implements slice 2b.
5. **`NonNullEqref`/`NonNullStructRefAny` `ValueType` variants** beyond
   whichever one(s) the corpus actually proves necessary for Correction
   3 — don't build untested surface speculatively.

## Recommended slice decomposition (dependency-ordered, each independently corpus-verifiable)

1. **Slice 1 — `ref.eq`** (`0xD3`, base opcode space). Fully isolated.
   Closes `ref_eq.wast`'s 83 NYS to 0 on its own (no other slice touches
   anything `ref_eq.wast` needs).
2. **Slice 2 — `ref.test`/`ref.cast` extension** (concrete struct/array
   nominal casting + abstract-heap-type structural testing). Closes most
   of `ref_test.wast` (71) and `ref_cast.wast` (45), plus `i31.wast`'s 4
   in-scope directives.
3. **Slice 3 — `any.convert_extern`/`extern.convert_any`** (`0xFB 0x1A`/
   `0x1B`). Independent of slices 1/2/4; closes the remainder of `ref_
   test.wast`/`ref_cast.wast` that slice 2 alone doesn't (both files use
   `any.convert_extern` early in their fixture setup, gating everything
   after it).
4. **Slice 4 — `br_on_cast`/`br_on_cast_fail`** (`0xFB 0x18`/`0x19`, plus
   the `NonNullAnyref`-family prerequisite). Depends on slice 2's heap-
   type-immediate encoder/decoder existing (reused verbatim for `ht1`/
   `ht2`) and on the `Correction 3` `ValueType` addition. Closes `br_on_
   cast.wast` (31) and `br_on_cast_fail.wast` (31).
5. **Slice 5 — full corpus re-verification**, per the "Design" section
   above.

Each slice should land as its own PR with its own real corpus-delta
verification (the same "quote the exact before/after NYS count for the
files this slice touches" discipline every W32-W38 slice already
follows), not a single combined PR — this cluster's own internal
causes are independent enough (Correction 1/2/3) that bundling them would
make a broken slice much harder to bisect.

## Verification plan (for whichever session implements this)

1. Before starting, re-run this spec's own throwaway-probe method once
   more against current `main` — confirm the 265/307 split and the four
   distinct causes are still accurate (this campaign's own repeated
   experience: numbers drift as other PRs land in a fast-moving
   monorepo).
2. After each slice, `cargo run --release --bin wasm_conformance_report`
   and diff the per-file NYS counts against this spec's own tables above
   — every count should move exactly as predicted, no more, no less.
3. Add direct unit tests (not just corpus fixtures) for: `ref.eq` on two
   nulls, two equal i31 payloads, two DIFFERENT i31 payloads, an i31
   compared against a struct handle (must be `0`, not a trap); `ref.test`/
   `ref.cast` against a concrete struct type with a live struct of a
   DIFFERENT concrete struct type (must fail/return `0`, proving the
   nominal check actually discriminates, not just "any struct matches
   any struct" still); `br_on_cast`/`br_on_cast_fail` against a literal
   null operand with `ht2` nullable vs. non-nullable, asserting the
   branch is taken/not-taken in BOTH directions per the fetched rule.
4. Full `cargo test --workspace` (or the repo's own `build-tool`-driven
   affected-package run) after each slice — `wasm-types`'s `ValueType`
   addition in particular ripples into `is_assignable`'s subtyping
   lattice, `wasm-runtime`, and potentially `wasm-module-encoder`, all of
   which have their own existing test suites that must keep passing.
5. Final aggregate check: total corpus NYS drops by exactly 265 (not 307)
   relative to the pre-W39 baseline, with zero new `Fail`/regressions
   anywhere in the 257-file corpus.
