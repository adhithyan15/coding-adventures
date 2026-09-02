# W37 — WASM GC reftype tables (table declarations beyond funcref/externref)

## Purpose and how this slice was chosen

`code/specs/W36-wasm-element-segment-exprs-list.md`'s own corpus-impact
table, in its "GC proposal remainder" row (`ref_eq.wast`, `ref_test.wast`,
`ref_cast.wast`, `i31.wast`, `br_on_cast.wast`, `array.wast`/`array_copy.
wast`/`array_init_data.wast`/`array_init_elem.wast`/`array_new_data.wast`/
`array_new_elem.wast`/`array_fill.wast`, `struct.wast`, `type-subtyping.
wast`, `type-rec.wast`, `table-sub.wast`, `ref.wast`, `call_ref.wast`,
`return_call_ref.wast`, "~500 combined per Addendum 2"), diagnosed this
cluster as "**Dominated by a separate, much larger, unrelated gap**: table
DECLARATIONS in this repo only accept `funcref`/`externref` as their
element type (`wasm-wast-parser/src/module.rs:1343-1352`, `2057`, `2118`
all reject any other reftype keyword with `"expected funcref or
externref, found ..."`) — `anyref`/`i31ref`/`structref`/`eqref`-typed
tables are rejected before element-segment parsing is ever reached," and
explicitly deferred a full spec-and-fix pass to "a future session."

This document is that pass. Per this campaign's own standing discipline —
every W32-W36 spec re-verified its own motivating claim directly against
the pinned corpus and current source before trusting it, and every one of
them found at least one thing the motivating document got wrong or
stale — this spec re-derives the table-declaration gap's real current
scope from scratch (throwaway-probe method: a temporary `wasm-conformance`
example calling `run_wast_source` on each file, reading every distinct
`NotYetSupported` message and slicing the exact source substring at its
reported byte position) rather than trusting W36's characterization.

**The re-verification confirms the table-declaration gap is real, gives it
a precise and narrow fix, and finds the fix's own de-risking is unusually
favorable (the runtime storage and validator layers already generalize
for free) — but it also finds W36's own "dominated by" framing needs a
significant correction**, in three independent ways, detailed below.

## Correction 1: the cluster's current total is ~550, not "~500+," and it splits into three, not one, root causes

Direct probe of the current `main`-tip source (`93d2e914fb2c0f7f7ea6155
eee936225f44b519e`, the tip origin/main was on at the time this spec was
written), same throwaway-probe method as every prior W-spec in this
series, against every file W36's own table names:

| File | Total NYS | Cause |
|---|---:|---|
| `ref_eq.wast` | 83 | 100% table-decl (`(table 20 (ref null eq))`) |
| `ref_test.wast` | 71 | 100% table-decl (`anyref`, `(ref null struct)`) |
| `ref_cast.wast` | 45 | 100% table-decl (`anyref`, `(ref null struct)`) |
| `i31.wast` | 46 | 42 table-decl (`i31ref`, `anyref`, `(ref i31)`); 4 separate (`ref.cast`'s own abstract-heap-type-immediate restriction) |
| `br_on_cast.wast` | 31 | 30 table-decl (`anyref`, `structref`); 1 separate (`(ref any)` in a func param/result position) |
| `br_on_cast_fail.wast` | 31 | 30 table-decl; 1 separate (same `(ref any)` gap) |
| `table-sub.wast` | 3 | 1 table-decl (`(ref null func)`/`(ref null $t)`); 2 pre-existing (no type-checker) |
| `array.wast` | 28 | **0 table-decl** — `array.new_data` unimplemented (14), elem-segment compound-reftype disambiguation (13, see Correction 2), `(ref struct)` storage-type gap (1) |
| `array_copy.wast` | 31 | **0 table-decl** — `array.copy` unimplemented |
| `array_fill.wast` | 27 | **0 table-decl** — `array.fill` unimplemented |
| `array_init_data.wast` | 44 | **0 table-decl** — `array.init_data` unimplemented |
| `array_init_elem.wast` | 33 | **0 table-decl** — `array.init_elem` unimplemented (20); bare `arrayref` value-type keyword unrecognized (13) |
| `array_new_data.wast` | 28 | **0 table-decl** — `array.new_data` unimplemented |
| `array_new_elem.wast` | 24 | **0 table-decl** — `array.new_elem` unimplemented (11); bare `i31ref`/`arrayref` in elem-segment disambiguation (13, see Correction 2) |
| `struct.wast` | 1 | **0 table-decl** — pre-existing (no type-checker) |
| `type-subtyping.wast` | 13 | **0 table-decl** — pre-existing (no type-checker) + a narrow, unrelated `(ref func)`/`(ref any)` non-null-abstract-heap-type gap |
| `type-rec.wast` | 3 | **0 table-decl** — pre-existing (no type-checker) |
| `ref.wast` | 3 | **0 table-decl** — pre-existing (no type-checker) + the same `(ref func)` gap |
| `call_ref.wast` | 2 | **0 table-decl** — pre-existing (no type-checker) |
| `return_call_ref.wast` | 3 | **0 table-decl** — pre-existing (no type-checker) + the same `(ref func)` gap |
| **Total** | **550** | **302 table-decl-attributable; 215 array-bulk-op-instruction-attributable (unrelated); 33 pre-existing/other (unrelated)** |

Every row's cause was confirmed by reading the exact `NotYetSupported`
message and the exact source slice at its reported byte position, not
inferred from the file name. Two representative examples (full probe
output is longer):

```text
ref_eq.wast, byte 290 (module's own FIRST table declaration):
  "at byte 290: expected funcref or externref, found "list""
  source: ...(table 20 (ref null eq))...

array_copy.wast, byte 1769 (the file's dominant construct, 31 hits):
  "at byte 1769: unknown instruction "array.copy""
  source: ...(array.copy $arr8_mut $arr8 (ref.null $arr8_mut) ...
```

**`array.wast` and its six `array_*.wast` siblings (215 of the cluster's
550 NYS, 39%) are NOT blocked by table declarations at all** — every one
of their fixtures either declares no table, or declares an ordinary
`funcref` table that already parses fine. Their real, unrelated cause is
that `array.copy`/`array.fill`/`array.init_data`/`array.init_elem`/
`array.new_data`/`array.new_elem` (the GC array bulk-operations proposal's
own instructions) are simply not implemented in `wasm-wast-parser`'s
instruction encoder at all — `grep` for any of their names in `module.rs`
finds nothing. This is a separate, comparably-sized future item (see
"Explicitly out of scope" below), not part of this spec.

**`struct.wast`/`type-rec.wast`/`type-subtyping.wast`/`call_ref.wast`/
`return_call_ref.wast`/`ref.wast` (25 of the 550, 5%) are ALREADY almost
entirely fixed** — by the W32-W35 epic, well before this spec. Their tiny
remaining counts are the pre-existing, permanently-deferred "no
instruction-level type-checker" `assert_invalid` gap (`code/specs/
W05-wasm-conformance-harness.md` §4.3) plus one small, genuinely separate
`parse_value_type` gap (a non-null ABSTRACT heap type, `(ref func)`/`(ref
any)`, still has no `ValueType` representation — a pre-existing,
deliberate scope boundary documented at `module.rs`'s own `parse_value_
type` doc comment, unrelated to table declarations). W36's "GC proposal
remainder" framing implicitly treated this whole named-file list as one
undifferentiated bucket; it is not.

## Correction 2: a second, unrelated gap hides inside the SAME "unknown instruction" pile — bare `i31ref`/`arrayref` in an elem segment's own reftype position

`array_new_elem.wast` and part of `array.wast`'s own NYS trace to
`(elem $e i31ref (ref.i31 (i32.const 0xaa)) ...)`/`(elem $bvec (ref
$bvec) (array.new $bvec ...) ...)` — an element segment whose exprs-list
reftype keyword is `i31ref` (a bare atom) or `(ref $bvec)` (a compound,
non-`funcref`/`externref` list). `build_elem`'s reftype-vs-funcidx-list
disambiguation (the same site W36's own item 3 already flagged for
`structref`/`(ref func)`-shaped compounds) only recognizes the bare atoms
`"funcref"`/`"externref"` — anything else, bare or compound, falls
through into being encoded as if it were the first FUNCIDX-LIST entry,
producing the confusing `"unknown instruction \"i31ref\""`/`"unknown
instruction \"ref\""` messages this probe found (the parser tries to
encode `i31ref`/`ref` as an instruction mnemonic once the reftype check
fails to consume it). This is real, but it is an **elem-segment** gap,
not a **table-declaration** gap, and it lives entirely inside the
`array_*.wast` bucket already excluded above — flagged here only so a
future implementer doesn't mistake array_new_elem.wast's NYS count for
evidence against this spec's own numbers.

## Correction 3: W36's own "three call sites, same rejection" claim is now stale for one of them

W36 cited three call sites in `wasm-wast-parser/src/module.rs` — imported-
table (`1343-1352`), declared-table "limits reftype" form (`2057`), and
declared-table "reftype (elem e*)" inline-shorthand form (`2118`) — as all
rejecting any non-`funcref`/`externref` reftype with the identical
message. Direct read of the CURRENT source (line numbers shifted slightly
since W36; re-verified live) finds this is no longer true for the third
site:

```rust
// module.rs:1343-1352 (imported table) -- UNCHANGED since W36:
let element_type = match reftype.as_atom() {
    Some("funcref") => wasm_types::FUNCREF,
    Some("externref") => wasm_types::EXTERNREF,
    _ => return Err(WastParseError::UnexpectedToken {
        pos: reftype.pos(), found: reftype.as_atom().unwrap_or("list").to_string(),
        expected: "funcref or externref",
    }),
};

// module.rs:2050-2059 (declared table, "limits reftype" form) -- UNCHANGED:
ctx.module.tables[storage_idx as usize].element_type = match reftype.as_atom() {
    Some("funcref") => wasm_types::FUNCREF,
    Some("externref") => wasm_types::EXTERNREF,
    _ => return Err(WastParseError::UnexpectedToken {
        pos: reftype.pos(), found: reftype.as_atom().unwrap_or("list").to_string(),
        expected: "funcref or externref",
    }),
};

// module.rs:2093-2122 (declared table, "reftype (elem e*)" inline-shorthand
// form) -- CHANGED since W36, by W32's own second slice:
let reftype = expect_get(rest, 0)?;
match reftype.as_atom() {
    Some("funcref") => {}
    Some("externref") => ctx.module.tables[storage_idx as usize].element_type = wasm_types::EXTERNREF,
    _ => match parse_value_type(reftype, &ctx.type_names, &ctx.module) {
        Ok(ValueType::Funcref) => {}
        Ok(ValueType::Externref) => ctx.module.tables[storage_idx as usize].element_type = wasm_types::EXTERNREF,
        Ok(vt @ (ValueType::ConcreteFuncRef(_) | ValueType::NonNullConcreteFuncRef(_))) => {
            ctx.module.tables[storage_idx as usize].element_type = wasm_types::FUNCREF;
            ctx.module.table_concrete_element_types[storage_idx as usize] = Some(vt);
        }
        Ok(other) => return Err(WastParseError::UnexpectedToken {
            pos: reftype.pos(), found: format!("{other:?}"),
            expected: "funcref, externref, or a concrete function reference type",
        }),
        Err(_) => return Err(WastParseError::UnexpectedToken {
            pos: reftype.pos(), found: reftype.as_atom().unwrap_or("list").to_string(),
            expected: "funcref or externref",
        }),
    },
}
```

The third site already dispatches through `parse_value_type` and already
accepts a **concrete function reference type** (`(ref $t)`/`(ref null
$t)` naming a func type) — real, working, corpus-exercised (`br_table.
wast`'s own `meet-funcref-*` tests) since W32's second slice. It rejects
everything else — including every GC reftype this spec is about — with an
upgraded three-way message, not W36's cited two-way one. **The first two
sites (import, and the far more commonly used "limits reftype" form) are
genuinely unchanged from W36's description** — and the "limits reftype"
form is what every table in this spec's own cluster actually uses
(`(table 20 (ref null eq))`, `(table $ta 10 anyref)`, `(table 20
structref)`, ...) — so W36's diagnosis of WHICH gap blocks this cluster
was right even though its own supporting evidence (all three sites
identical) was already one slice out of date when it was written.

## What the real spec requires (grounded in the real spec text)

Fetched directly from `https://webassembly.github.io/gc/core/text/types.
html`, `https://webassembly.github.io/gc/core/syntax/types.html`, and
`https://webassembly.github.io/gc/core/binary/types.html` (quoted
verbatim, not paraphrased):

**Text-format grammar:**
```text
heaptype ::= absheaptype | typeidx
absheaptype ::= func | nofunc | extern | noextern | any | eq | i31 | struct | array | none
reftype ::= (ref ht:heaptype) | (ref null ht:heaptype)
tabletype ::= limits reftype
```

**Abbreviations** (each ≡ `(ref null <heaptype>)`):
```text
funcref ≡ (ref null func)        nullfuncref   ≡ (ref null nofunc)
externref ≡ (ref null extern)    nullexternref ≡ (ref null noextern)
anyref ≡ (ref null any)          nullref       ≡ (ref null none)
eqref ≡ (ref null eq)
i31ref ≡ (ref null i31)
structref ≡ (ref null struct)
arrayref ≡ (ref null array)
```

**Binary encoding** (single-byte abstract heap types, doubling as the
nullable reftype shorthand):
```text
0x73 nofunc   0x72 noextern   0x71 none   0x70 func   0x6F extern
0x6E any      0x6D eq         0x6C i31    0x6B struct 0x6A array
```
Compound forms: `0x64 <heaptype>` = `(ref <heaptype>)` (non-null), `0x63
<heaptype>` = `(ref null <heaptype>)` (nullable) — a concrete `<heaptype>`
is a `typeidx`.

**A table's element type is grammatically just `reftype` — the spec places
NO restriction on which reference type a table may hold.** Every abstract
heap type and every concrete type index is equally valid there; this
repo's own restriction to `funcref`/`externref`(/concrete-func, since W32)
is purely a self-imposed implementation gap, not something the spec
narrows for tables specifically.

## Current implementation, read directly

### `wasm-types/src/lib.rs`: which GC `ValueType` variants already exist

Confirmed by direct read: `Funcref`, `Externref`, `Anyref`, `I31ref`,
`Exnref`, `NullRef`, `NullFuncref`, `NullExternref`, `NullExnref`,
`ConcreteFuncRef(u32)`/`NonNullConcreteFuncRef(u32)` (nullable/non-null
concrete function reference), `StructRef(u32)`/`NonNullStructRef(u32)`
(nullable/non-null concrete STRUCT reference), `ArrayRef(u32)`/
`NonNullArrayRef(u32)` (nullable/non-null concrete ARRAY reference),
`NonNullArrayAny` (non-null reference to the abstract top of the array
hierarchy, i.e. `(ref array)`).

**Missing entirely**: `eqref`/`(ref null eq)`/`(ref eq)` — no `Eqref`
variant, and `"eq"`/`"eqref"` appear nowhere in `wasm-types` or `wasm-
wast-parser`. `structref`/`(ref null struct)`/`(ref struct)` as the
ABSTRACT top of the struct hierarchy — `StructRef(u32)` is a nullable
CONCRETE reference (always carries a type index); there is no nullable
abstract-struct-top variant (the array hierarchy already has this
asymmetry solved one direction: `NonNullArrayAny` exists for `(ref
array)`, but there is no nullable `ArrayRefAny` counterpart for bare
`arrayref`/`(ref null array)` either — not needed by any TABLE
declaration in this cluster, see "Explicitly out of scope").

### `wasm-wast-parser/src/module.rs`: `parse_value_type`'s heap-type dispatch (lines 310-430)

The bare-atom match (line ~391-428) recognizes `funcref`, `externref`,
`i31ref`, `anyref`, `nullref`, `nullfuncref`, `nullexternref`,
`nullexnref`, `exnref` — **not** `eqref`, `structref`, or `arrayref`. The
compound `(ref [null] <heaptype>)` list branches (line ~352-384)
special-case `func`/`extern`/`i31` (3-item `null` form) and `i31`/`array`
(2-item non-null form) before falling through to `concrete_ref_value_
type` (a real, already-correct dispatch on `module.type_kinds` that
produces `StructRef`/`ArrayRef`/`ConcreteFuncRef` as appropriate,
confirmed by direct read of lines 265-308) — **not** `eq`, `struct`, or
`any` in either branch. This is why `(ref null eq)`/`structref`/`(ref
null struct)` all fail here today, and it is the SAME function every
other value-type-typed position in this crate already reuses (params,
results, locals, globals, `ref.test`/`ref.cast`'s own type immediate) —
fixing it here is shared infrastructure, not a table-specific patch.

`concrete_ref_value_type` and the two GC-storage-type-adjacent functions
that already dispatch generically prove the REPRESENTATION side is not
the blocker: this crate can already produce `StructRef`/`ArrayRef` for a
NAMED concrete type. The blocker is purely that `parse_value_type` never
reaches that dispatch for the ABSTRACT keywords `eq`/`struct` (and, for
this spec's purposes, `any` inside a table-decl-adjacent 2-item `(ref
any)` position is also currently unhandled — but no table declaration in
this cluster needs `(ref any)` specifically, only the bare `anyref`
abbreviation, which the atom match already has; see "Explicitly out of
scope" for why non-null `(ref any)` stays unaddressed here).

### `wasm-types::WasmModule::table_concrete_element_types` — already fully generic

```rust
pub table_concrete_element_types: Vec<Option<ValueType>>,
```

confirmed at `wasm-types/src/lib.rs:1644`. **The field's own doc comment
currently asserts (line ~1614-1617) that `Some(vt)` is "always
`ConcreteFuncRef`/`NonNullConcreteFuncRef`... because no struct/array-
typed table can arise here"** — true only because nothing populates it
with anything else YET, not because the `Vec<Option<ValueType>>` type
itself is restricted. This doc comment will be WRONG the moment this
spec's design lands and must be rewritten as part of it (flagged
explicitly so the implementing session doesn't leave a stale, load-
bearing-sounding claim behind — the same class of drift this campaign's
own `feedback_prose_outruns_code_in_reviews` lesson warns about).

### `wasm-validator/src/type_check.rs`: `table_element_types` — already fully generic, zero changes needed

`build_module_context` (lines 978-1005) builds `table_element_types:
Vec<ValueType>` by, for each module-defined table, preferring `module.
table_concrete_element_types.get(i).copied().flatten()` and falling back
to a byte-tag-based `Funcref`/`Externref` guess only when that's `None`.
This is **already** exactly the generic mechanism this spec's fix needs —
whatever `ValueType` the parser stores in `table_concrete_element_types`,
the validator's `table.get`/`table.set`/`table.fill`/`table.copy`/
`call_indirect` type-checking (which all consume `table_element_types`
downstream, confirmed by the same file's own opcode arms) will type-check
against it with **no changes to this crate at all**. This is the single
biggest reason this spec's fix is smaller than its motivating framing
suggested: the type-checking side of "GC reftype tables" was already
built, generically, as an unplanned side effect of W32's concrete-
function-reference-type slice — it just has nothing but `Funcref`/
`Externref`/`ConcreteFuncRef`/`NonNullConcreteFuncRef` ever written into
it today.

### `wasm-execution/src/lib.rs`: `Table`/`TableStorage`/`TableElement` — already fully generic, zero changes needed

`TableElement` (lines 1483-1508) is a two-variant enum: `Raw(u32)` (an
unresolved/opaque payload) or `Func(FuncRefTarget)` (a resolved,
cross-instance-safe function reference, W35). Its own doc comment already
states, independent of and prior to this spec: *"`Raw(u32)`... is also
the ONLY variant a non-funcref (externref/GC-typed) table entry ever
uses"* — this repo's `Table`/`TableStorage` was already designed, during
the W35 epic, to be reference-type-agnostic; it holds a raw `u32` GC-heap
handle for an externref table today with zero type awareness, and a GC
struct/array/i31/anyref entry needs exactly the same shape. **No changes
needed here either.**

The one place that DOES care about a table's byte-level `element_type` is
`wasm-runtime`'s `combined_table_element_type`/`resolve_all_table_
funcrefs` (the W35 cross-instance-function-identity fixup pass,
`wasm-runtime/src/lib.rs:1663-1685`): it gates strictly on `element_type
== FUNCREF_ELEMENT_TYPE (0x70)` to decide whether to attempt resolving a
table's `Raw` entries into real `FuncRefTarget`s. This is why the fix must
keep `element_type` set to something other than `0x70` (matching the
existing convention: a concrete-func table already keeps `element_type =
FUNCREF` as a DELIBERATE placeholder since "every concrete function
reference is funcref-family" — the mirror-image choice for a genuinely
non-func GC table is to reuse `EXTERNREF` (`0x6F`) as the generic "opaque,
not-funcref" placeholder byte, exactly like an ordinary externref table
already does, so this fixup pass continues to skip it correctly with zero
changes to `wasm-runtime` either).

### `wasm-module-parser/src/lib.rs`: binary format — not needed, but a real latent gap noted

`parse_table_section` (lines 987-1031) reads `element_type` as one raw,
UNVALIDATED byte and stores it verbatim (the only rejection is the
unrelated `TABLE_WITH_INIT_EXPR_TAG` / `0x40` case). It never rejects an
`0x6E`/`0x6C`/anything-else byte, but it also never correctly decodes a
COMPOUND reftype (`0x63`/`0x64 <heaptype>`) — it would silently leave the
heap-type's own bytes unconsumed, corrupting the following `limits` read
(the same class of bug the `0x40` case's own doc comment describes for a
different tag). **Confirmed by direct grep of every file in this
cluster: none of them use `(module binary ...)`** — this asymmetry is
real but dormant, exercises no corpus file, and is explicitly left alone
here (see "Explicitly out of scope").

## Design

### 1. `wasm-types`: two new `ValueType` variants

- `Eqref` — nullable, abstract top of the `eq` hierarchy (`eqref`/`(ref
  null eq)`). Binary tag `0x6D` (`byte_tag()`/`encode()`, mirroring
  `Anyref`'s `0x6E` exactly).
- `StructRefAny` — nullable, abstract top of the `struct` hierarchy
  (`structref`/`(ref null struct)`), distinct from the existing
  `StructRef(u32)` (nullable, CONCRETE, always carries a type index).
  Binary tag `0x6B`, mirroring `Anyref`/`I31ref`'s single-byte shape.
  (Naming choice: mirrors this crate's own existing `NonNullArrayAny`
  precedent — "Any" suffix marks "the whole hierarchy's abstract top,"
  not a specific index.)

Wire both into `byte_tag()`/`encode()` exactly like `Anyref`/`I31ref`
already are (single byte, no LEB128 tail). Extend `is_bottom_subtype_of`
with `NullRef <: Eqref` and `NullRef <: StructRefAny` (mirroring the
existing `NullRef <: Anyref`/`NullRef <: I31ref`/`NullRef <: StructRef(_)`
arms exactly). The implementing session must also locate and mirror
whatever existing mechanism already makes `I31ref`/`StructRef(_)`
assignable to `Anyref` (i31.wast's non-table GC content already passes
today, so this edge is handled somewhere in `wasm-validator`'s
`is_assignable`/type-checking, not necessarily in `wasm-types` itself) and
extend it symmetrically for `Eqref <: Anyref` and `StructRefAny <: Eqref
<: Anyref` — this spec does not claim to have traced every consuming
call site of that mechanism; verify against `type-subtyping.wast`'s own
"Any hierarchy" subtyping cases as the ground truth.

Update `table_concrete_element_types`'s own doc comment (currently
claims "always `ConcreteFuncRef`/`NonNullConcreteFuncRef`... because no
struct/array-typed table can arise here") to describe the generalized
contract this spec's design section 2 below establishes.

### 2. `wasm-wast-parser`: extend `parse_value_type`'s heap-type dispatch

Two small, localized additions to the SAME function every other
value-type position already shares (no new function, no duplicated
grammar):

- Atom match (line ~391-428): add `"eqref" => Ok(ValueType::Eqref)`,
  `"structref" => Ok(ValueType::StructRefAny)`.
- Compound list branches (line ~352-374): add `"eq"` alongside `"func"`/
  `"extern"`/`"i31"` in the 3-item `(ref null <heaptype>)` branch, and
  `"struct"` alongside `"i31"`/`"array"` in the appropriate branch(es),
  each returning `Ok(ValueType::Eqref)`/`Ok(ValueType::StructRefAny)`
  respectively for BOTH the null and non-null spellings (matching this
  crate's own existing `i31`-handling precedent: no distinct non-null
  variant exists or is needed for either, since the corpus never
  exercises a non-null `eq`/`struct` — confirmed by this spec's own
  probe: only `(ref null eq)`/`(ref null struct)`/bare `eqref`/`structref`
  appear, never bare `(ref eq)`/`(ref struct)`).

`parse_ref_null_heap_type` (the SEPARATE function backing `ref.null
<heaptype>`'s own instruction immediate, lines ~442-495) needs the
identical two additions (`"eq" => Ok(vec![0x6D])`, `"struct" =>
Ok(vec![0x6B])`) — confirmed needed by direct probe: `ref_eq.wast`'s own
`(ref.null eq)` and `ref_cast.wast`/`ref_test.wast`/`br_on_cast*.wast`'s
own `(ref.null struct)` calls (used as ordinary STACK OPERANDS, not table
declarations) exercise this function, not `parse_value_type`, and would
otherwise still fail after this spec's table-decl fix alone.

### 3. `wasm-wast-parser`: generalize the "limits reftype" table-declaration site

`build_table_limits_and_elements`'s "starts_with_limit_number" branch
(module.rs, currently lines ~2038-2062) — the form used by EVERY table
declaration in this spec's own cluster (`(table 20 (ref null eq))`,
`(table $ta 10 anyref)`, `(table 20 structref)`, `(table $t1 10 (ref null
func))`) — currently hand-rolls a bare `"funcref"`/`"externref"` atom
match with no `parse_value_type` call at all. Replace it with the
IDENTICAL dispatch the inline-shorthand site (§3 above, "Correction 3")
already established, but WITHOUT that site's narrow allowlist — accept
any successfully-parsed `ValueType`, not just `Funcref`/`Externref`/
`ConcreteFuncRef`/`NonNullConcreteFuncRef`:

```rust
let reftype = expect_get(rest, digit_count)?;
match parse_value_type(reftype, &ctx.type_names, &ctx.module) {
    Ok(ValueType::Funcref) => {} // already the default placeholder
    Ok(ValueType::Externref) => ctx.module.tables[storage_idx as usize].element_type = wasm_types::EXTERNREF,
    Ok(other) => {
        // Any richer reference type (concrete func/struct/array, or an
        // abstract GC top type) is representable generically -- see
        // `table_concrete_element_types`'s own (updated) doc comment.
        // `element_type`'s own byte only needs to stay OFF the funcref
        // fast path so `wasm-runtime`'s cross-instance-funcref fixup
        // pass (W35) doesn't try to resolve a non-func handle as a
        // function index -- funcref-FAMILY concrete types keep FUNCREF
        // (existing convention), everything else gets EXTERNREF
        // (matches how an ordinary externref table is already treated).
        let is_func_family = matches!(other, ValueType::ConcreteFuncRef(_) | ValueType::NonNullConcreteFuncRef(_));
        ctx.module.tables[storage_idx as usize].element_type =
            if is_func_family { wasm_types::FUNCREF } else { wasm_types::EXTERNREF };
        ctx.module.table_concrete_element_types[storage_idx as usize] = Some(other);
    }
    Err(_) => return Err(WastParseError::UnexpectedToken {
        pos: reftype.pos(), found: reftype.as_atom().unwrap_or("list").to_string(),
        expected: "a reference type",
    }),
}
```

This is a strict SIMPLIFICATION relative to today's code (one match arm
covering every case, versus today's binary allowlist), not new surface
area — it reuses `parse_value_type` (already shared infrastructure once
§2 above lands) and `table_concrete_element_types` (already shared
infrastructure per the validator finding above) end to end. The inline-
shorthand site's own narrower `Ok(vt @ (ConcreteFuncRef|NonNullConcreteFuncRef))`
match arm (§ Correction 3) should be generalized identically, for
consistency and so a future `(table funcref (elem ...))`-shaped GC table
doesn't hit a second, differently-scoped restriction — though no file in
THIS cluster currently exercises that combination, so this is a
consistency cleanup, not a corpus-driven requirement.

**Left unchanged, deliberately**: the imported-table site (module.rs
~1343-1352). Confirmed by grep of the entire pinned corpus: no file
imports a GC-reftype table. `wasm-validator`'s own existing doc comment
(`type_check.rs:978-981`) already documents this as a standing, deliberate
scope boundary — "Import tables can only ever be generic funcref/
externref in this crate's text format (no concrete-typed table IMPORT
syntax exists...)" — extending it would need a NEW parallel mechanism for
imports (the current `table_concrete_element_types` vec is explicitly
module-defined-tables-only, per its own doc comment), not a small change,
and nothing in the corpus asks for it.

### 4. Binary format: no change

Confirmed by corpus grep: no file in this cluster uses `(module binary
...)`. Left alone per "Explicitly out of scope" below.

### 5. `wasm-validator`/`wasm-execution`: no changes

Both already generalize for free — see "Current implementation" above.

## Does this unblock the downstream features these files test? (re-verified per-file, not assumed)

This is the question W36's own "re-probe before assuming a bucket
evaporates" discipline (already applied once in that spec, to `bulk.wast`)
demands here too. The honest answer, confirmed by checking whether each
file's OWN post-table-declaration content is otherwise implemented:

- **`ref_cast.wast` (45 NYS) — the most likely to reach real `Pass`.**
  Its own `ref.cast`/`ref.test` invocations always target a concrete
  `$t0`/`$t1`/... type (never an abstract one), so they do NOT hit the
  abstract-heap-type-immediate restriction below. Once its table
  declarations parse, nothing else in this file is currently known to be
  missing — but this is a prediction, not a re-verified fact; re-probe
  after slice 3 lands.
- **`table-sub.wast` (1 of 3 NYS)** — closes; its other 2 are the
  pre-existing no-type-checker gap, unaffected either way.
- **`ref_test.wast` (71 NYS) — will parse further but will NOT reach
  `Pass`.** It calls `extern.convert_any` (line 34), confirmed by grep to
  be entirely unimplemented in `wasm-wast-parser` (no match for the name
  anywhere). This is a separate, out-of-scope instruction gap.
- **`ref_eq.wast` (83 NYS) — will parse further but will NOT reach
  `Pass`.** `ref.eq` itself — this file's entire subject — is confirmed
  entirely unimplemented (no match anywhere in `wasm-wast-parser`/
  `wasm-execution`/`wasm-validator` beyond an unrelated comment). Every
  one of this file's `assert_return` cases invokes `ref.eq` directly, so
  fixing only the table declaration converts "module fails to parse" into
  "module fails to parse at `ref.eq` instead" — a different error
  message, not incremental progress toward `Pass`.
- **`br_on_cast.wast`/`br_on_cast_fail.wast` (30 NYS each) — will parse
  further but will NOT reach `Pass`.** `br_on_cast`/`br_on_cast_fail`
  themselves are confirmed entirely unimplemented (no match anywhere in
  `wasm-wast-parser`). Same shape as `ref_eq.wast`: this spec's fix moves
  the failure point, it does not close it.
- **`i31.wast` (42 of 46 NYS) — partial.** 38 (bare `i31ref`/`anyref`
  table declarations) plausibly resolve. The remaining 4 (`(table $t 3 3
  (ref i31) (ref.i31 (global.get $g)))`) need BOTH this spec's compound-
  reftype dispatch AND the separate, already-flagged-by-W36 "table with an
  explicit init expression (function-references proposal)" third table
  form (`TABLE_WITH_INIT_EXPR_TAG`/`0x40` in binary; the analogous TEXT
  form is a trailing initializer after `[limits] reftype` with no `(elem
  ...)` wrapper) — not implemented by EITHER W36 or this spec. Separately,
  the file's own `(ref.cast i31ref (global.get $c))` (4 NYS, already
  outside the table-decl count) hits `ref.test`/`ref.cast`'s existing
  restriction to concrete type immediates only (`module.rs:4397-4406`'s
  own doc comment claims "no vendored corpus case needs an abstract heap
  type for `ref.test`/`ref.cast` specifically" — **this claim is now also
  stale**, confirmed by this exact probe hit; flagged for whoever attempts
  that follow-on, not fixed here).

**Bottom line, honest and re-verified**: of the cluster's 550 current NYS
directives, this spec's fix is grounded in and directly addresses ~302
(the ones whose failure trace to a table declaration). Of those, roughly
45-46 (`ref_cast.wast` + `table-sub.wast`, pending re-verification) are
likely to convert to real `Pass`; the remaining ~256 (`ref_eq.wast`,
`ref_test.wast`, `br_on_cast.wast`, `br_on_cast_fail.wast`, most of
`i31.wast`) will parse further but reveal genuinely separate, already-
identified, entirely-unimplemented-instruction gaps (`ref.eq`,
`extern.convert_any`/`any.convert_extern`, `br_on_cast`/`br_on_cast_fail`,
the table-with-init-expression form, `ref.cast`'s abstract-heap-type
restriction) that this spec explicitly does not attempt. This is a much
smaller realized number than the task's own "500+" framing implied, and
smaller even than the "~302 table-decl-attributable" framing might
suggest at first glance — but it is the honestly re-verified number, not
an optimistic one, matching exactly what this campaign's own W35/W36
precedent established as the expected shape of this kind of
re-verification pass.

## Explicitly out of scope for this spec

- **The GC array bulk-operations instructions** (`array.copy`/`array.
  fill`/`array.init_data`/`array.init_elem`/`array.new_data`/`array.
  new_elem`) — confirmed to be this cluster's single largest bucket (215
  NYS, 39%), entirely unrelated to table declarations, and entirely
  unimplemented. A large, separate, natural follow-on spec (parsing +
  validation + execution for six real instructions) — candidate title
  "GC array bulk operations."
- **The elem-segment bare-reftype-keyword gap for `i31ref`/`arrayref`**
  (Correction 2 above) — lives inside the array bulk-ops bucket just
  above; fix alongside it, not here (it is a generalization of W36's own
  item 3, which is already that spec's own subject, not this one's).
- **The bare `arrayref` value-type keyword** (needed for `(array (mut
  arrayref))`-shaped storage types in `array_init_elem.wast`/`array_new_
  elem.wast`) and the corresponding nullable `ArrayRefAny` `ValueType`
  variant — real, but not needed by any TABLE declaration in this
  cluster (confirmed by corpus grep: no `(table ... arrayref ...)`/`(table
  ... (ref null array) ...)` anywhere). Bundle with the array bulk-ops
  follow-on instead.
- **Non-null abstract heap types** (`(ref any)`, `(ref func)`, `(ref
  extern)`) — a pre-existing, deliberate scope boundary this crate's own
  `parse_value_type` doc comment already documents ("a non-null ABSTRACT
  heap type still has no `ValueType` variant, only non-null CONCRETE ones
  do"). Real (confirmed live in `type-subtyping.wast`/`br_on_cast*.wast`'s
  func param/result positions), but not needed by any table declaration
  in this cluster — every table here uses either a bare abstract atom
  (always implicitly nullable per the abbreviation table above) or an
  explicit `(ref null <heaptype>)`, never bare non-null `(ref any)`.
- **`ref.eq`, `extern.convert_any`/`any.convert_extern`, `br_on_cast`/
  `br_on_cast_fail`** — confirmed entirely unimplemented instructions,
  each a real, separate, likely medium-to-large feature (parsing +
  validation + execution) in its own right. This spec's fix reaches the
  point where each becomes the NEXT blocker for its respective file, but
  implementing any of them is out of scope here.
- **`ref.test`/`ref.cast`'s restriction to concrete-only type
  immediates** (`module.rs:4397-4406`) — confirmed stale by this spec's
  own probe (`i31.wast`'s `(ref.cast i31ref ...)`), but narrow, separate,
  and not needed by `ref_test.wast`/`ref_cast.wast`'s own invocations
  (which always target a concrete `$t`). Flagged for a future session;
  the doc comment's own false "no vendored corpus case needs this" claim
  should be corrected as part of whichever future patch touches it.
- **The "table with an explicit init expression" third table form**
  (`TABLE_WITH_INIT_EXPR_TAG`/binary `0x40`, and its text-format
  equivalent) — already flagged out of scope by W36 for `elem.wast`'s own
  6 NYS of the identical shape; this spec's own probe confirms `i31.wast`
  needs the SAME unimplemented feature (4 NYS). Still out of scope; a
  natural third data point for whoever eventually specs it.
- **Binary-format table-type decoding** — confirmed unexercised by any
  file in this cluster. The dormant asymmetry noted above (unvalidated
  byte, no compound-reftype support) is real but should be fixed
  alongside whatever future work first needs a binary-encoded GC-typed
  table, not spuriously here.
- **GC-reftype-typed table IMPORTS** — confirmed unexercised by any file
  in this cluster, and would need a new mechanism (no existing "imported
  table's concrete type" field at all), not a small extension.

## Recommended slice decomposition

0. **`wasm-types`**: add `Eqref`/`StructRefAny`, wire `byte_tag()`/
   `encode()`, extend `is_bottom_subtype_of` with the two new `NullRef
   <:` edges, locate and mirror whatever existing mechanism makes
   `I31ref`/`StructRef(_)` assignable to `Anyref` for the two new types.
   Update `table_concrete_element_types`'s doc comment (still describes
   the PRE-slice-2 contract at this point — safe, since nothing yet
   writes anything but func-family into it). Verify: `cargo test -p
   wasm-types` clean; new unit tests for `byte_tag()`/`encode()`/
   `is_bottom_subtype_of` on both new variants, mirroring `Anyref`'s own
   existing test shapes exactly.
1. **`wasm-wast-parser`**: extend `parse_value_type` (atom match +
   compound list branches) and `parse_ref_null_heap_type` for `eq`/
   `struct`/`eqref`/`structref`. Verify: `cargo test -p wasm-wast-parser`
   clean; new unit tests for `(ref null eq)`, `(ref null struct)`, bare
   `eqref`, bare `structref`, and `(ref.null eq)`/`(ref.null struct)`
   instruction immediates — independently testable before touching any
   table-declaration code, since every other `parse_value_type`/`parse_
   ref_null_heap_type` call site benefits identically.
2. **`wasm-wast-parser`**: generalize the "limits reftype" table-
   declaration branch (§3 above) and, for consistency, the inline-
   shorthand branch's existing narrow allowlist. Update `table_concrete_
   element_types`'s doc comment for real this time. Verify: `cargo test -p
   wasm-wast-parser`; new unit tests for every corpus-confirmed shape —
   bare `i31ref`/`anyref`/`structref`, compound `(ref null eq)`/`(ref null
   struct)`/`(ref null func)`/`(ref null $t)`/`(ref i31)` — each asserting
   both that the module parses AND that `table_concrete_element_types`
   holds the expected `ValueType`.
3. **Full corpus re-verification.** Re-run `wasm_conformance_report
   --write-baseline`, diff against the pre-slice-0 baseline. Confirm the
   PER-FILE predictions in "Does this unblock the downstream features"
   above precisely — NOT a blanket cluster-wide improvement claim:
   - `ref_cast.wast`/`table-sub.wast`: expect NYS to drop toward/to zero
     (real `Pass`, pending this re-verification).
   - `ref_eq.wast`/`ref_test.wast`/`br_on_cast.wast`/`br_on_cast_fail.
     wast`: expect the SAME NYS count (directives still not gradeable),
     but a DIFFERENT underlying message — confirm via the same probe
     method that the new blocker is exactly `ref.eq`/`extern.convert_any`/
     `br_on_cast`/`br_on_cast_fail` (an unknown-instruction error), not a
     silent regression or an unexpected new failure mode.
   - `i31.wast`: expect ~38 of its 46 NYS to newly resolve or reveal the
     init-expression/abstract-ref.cast blockers as predicted; re-probe
     precisely rather than trusting the aggregate delta.
   - Every OTHER file in the corpus (all 257, not just this cluster):
     confirm zero regressions — this spec's `parse_value_type`/`parse_
     ref_null_heap_type` changes are shared infrastructure touched by
     every value-type-typed position in the grammar, so a regression risk
     exists wherever `eq`/`struct` could ever have been (mis)parsed as
     something else before (unlikely, since both were previously hard
     parse errors, never silently misinterpreted — but confirm, don't
     assume).

## Verification plan (for whatever session implements this)

1. `cargo build --workspace` and `cargo test --workspace` (or at minimum
   `-p wasm-types -p wasm-wast-parser -p wasm-validator -p wasm-execution
   -p wasm-runtime -p wasm-conformance`) clean after EACH slice, not just
   at the end — this campaign's own established "verify the narrow piece
   before the wiring" precedent (W35/W36's own decomposition sections).
2. `cargo run --release --bin wasm_conformance_report -p wasm-conformance
   -- --write-baseline`, diffed against the pre-slice-0 baseline, checked
   against the EXACT per-file predictions above, not a blanket "cluster
   improved" claim.
3. For every corpus-cited fixture in this spec (`ref_eq.wast` line 10,
   `ref_test.wast` lines 8-10/192, `ref_cast.wast` lines 8/109, `i31.wast`
   lines 62/130/169, `br_on_cast.wast`/`br_on_cast_fail.wast` lines 8/114,
   `table-sub.wast` lines 3-4), spot-check with a direct `run_wast_source`
   probe (the same throwaway-example method this spec's own investigation
   used) rather than trusting the aggregate report number alone.
4. Confirm `table_concrete_element_types`'s generalized contract doesn't
   silently corrupt an EXISTING concrete-func table's own behavior —
   `br_table.wast`'s `meet-funcref-*`/`meet-multi-ref` tests (the ones
   W32's second slice originally fixed) must still pass unchanged; this
   spec's change touches the exact same code path they depend on.
5. Confirm `wasm-runtime`'s cross-instance-funcref fixup pass
   (`resolve_all_table_funcrefs`) still correctly SKIPS every new GC-typed
   table (verify `element_type` never reads back as `0x70` for one) —
   the same class of silent-wrong-value bug this pass's own doc comment
   already documents having been bitten by once, for externref.
