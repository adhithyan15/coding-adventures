# W34 — WASM GC canonical type-group equivalence

## Purpose and how this slice was chosen

`code/specs/W33-wasm-gc-recursive-type-subtyping.md` and all four of its
addenda converge, independently, on the same conclusion: **canonical
type-group equivalence** — recognizing that two separately-declared `(rec
...)` groups (even across different modules, sharing no type-section
numbering at all) declare "the same types" whenever their SHAPES match,
per the real WasmGC proposal's algorithm — is the one piece of real
semantic work this repo's WASM type system still lacks. The convergence,
traced through the actual addendum text:

- **First slice's addendum** named it "(3b)" and confirmed it necessary
  and sufficient for all 6 `type-equivalence.wast` fails and 2
  `type-subtyping.wast` `assert_unlinkable` fails (the `M10`/`M11`
  `rec`-group topology-mismatch pair).
- **Third slice's addendum** re-confirmed (3b) unchanged, and — this is
  the addendum's own load-bearing claim — mis-scoped it as strictly a
  CROSS-MODULE problem ("(3b) is for `type-equivalence.wast`'s
  cross-module cases, not `type-canon.wast`'s single-module ones").
- **Fourth slice's addendum** directly falsified that narrower framing,
  by direct trace rather than re-assumption: once struct/array TEXT-format
  parsing unlocked `type-rec.wast`'s "Static matching of recursive types"
  and "Dynamic matching of recursive types" sections, those turned out to
  need (3b) too — for validating a SINGLE module's OWN globals against its
  own two separately-declared, structurally-identical `rec` groups, no
  cross-module linking involved at all. The fourth addendum's own words:
  "a `rec`-group pair like `$f1`/`$f2` ... turns out to need canonical
  equivalence even for validating a SINGLE module's own OWN globals."

So (3b)'s reach is broader than any prior addendum assumed on first pass,
and narrower than "rewrite the whole type system" would suggest — it is
one well-defined algorithm (canonicalization + comparison), wired into a
small, enumerable set of call sites. This spec is that algorithm, cited
against its two authoritative sources (the GC proposal's own formal
prose, AND the reference interpreter's actual OCaml implementation — both
read directly for this spec, per the same discipline W24's `exnref` fix
already established for this campaign), plus a concrete plan for wiring
it into this crate's existing `wasm_types`/`wasm-validator`/
`wasm-execution`/`wasm-runtime` architecture.

## What already exists (grounded in the actual current code)

All claims below were checked directly against
`code/packages/rust/wasm-types/src/lib.rs`,
`code/packages/rust/wasm-validator/src/type_check.rs`,
`code/packages/rust/wasm-execution/src/lib.rs`,
`code/packages/rust/wasm-runtime/src/lib.rs`, and
`code/packages/rust/wasm-conformance/src/lib.rs` as they exist on this
branch (`wasm-types` 0.1.17, `wasm-validator` 0.2.79, `wasm-execution`
0.9.82, `wasm-runtime` 0.6.19, `wasm-conformance` 0.1.109 — the versions
the W33 fourth addendum shipped), not re-assumed from the addenda's prose.

### The data model (`wasm_types`)

`WasmModule` (`wasm-types/src/lib.rs:1523`) holds, per flat type-section
index (one slot per `rec`-group member, including singleton groups):

- `types: Vec<FuncType>` — a func-kind slot's real payload; a
  struct/array-kind slot holds an unused dummy `FuncType` (kept only to
  preserve "one slot per index").
- `type_kinds: Vec<TypeKind>` (`Func | Struct(u32) | Array(u32)`,
  `lib.rs:1035`-ish) — which of `types`/`struct_types`/`array_types`
  actually holds this index's payload (W33 fourth slice: needed once
  struct/array declarations can interleave with func ones in arbitrary
  source order).
- `struct_types: Vec<StructType>` / `array_types: Vec<ArrayType>` — real
  bodies, `StructType.fields: Vec<FieldType>`, `FieldType { storage:
  StorageType, mutable: bool }`, `StorageType { Val(ValueType) | I8 |
  I16 }`, `ArrayType { element: FieldType }`.
- `type_subtyping: Vec<TypeSubtyping>` (`lib.rs:936`) — `{ supertype:
  Option<u32>, is_final: bool, rec_group_size: u32, rec_group_position:
  u32 }`, one entry per flat index, defaulting to `(None, true, 1, 0)`
  (final, no parent, singleton group — the pre-GC/MVP shape) when a
  module never populates it.

**A stale doc comment, worth correcting for whoever picks this up**:
`TypeSubtyping`'s own doc comment (written in the W33 first slice, before
struct/array parsing existed) says it applies to "function types only,
this slice." That is no longer accurate — `wasm-wast-parser`'s phase-A
loop (`module.rs:944-998`) pushes a `TypeSubtyping` entry for EVERY
`rec`-group member regardless of `member_kind` (func, struct, or array;
confirmed by direct read, the push at `module.rs:968` sits outside the
`kind != MemberKind::Func` branch below it). `rec_group_size`/
`rec_group_position`/`supertype`/`is_final` are already tracked uniformly
across all three composite kinds; only the CONSUMERS of that data
(`is_assignable`, `check_type_subtyping`) remain func-only, a distinct,
narrower gap described below.

### Within-module nominal subtyping (what W33's first/third slices built)

`nominal_subtype_chain` (`wasm-types/src/lib.rs:1632`, a free function
over `&[TypeSubtyping]` rather than a `WasmModule` method, specifically
so `wasm-execution`'s `WasmExecutionContext` — which deliberately does
NOT hold a full `WasmModule`, see its own doc comments — can reuse the
identical walk): reflexive (`sub_idx == super_idx`), then walks
`supertype` links up to `MAX_SUBTYPE_CHAIN_HOPS` (1,000) hops, checking
for an EXACT index match at each step. `WasmModule::
func_type_is_nominal_subtype` is a thin wrapper. `any_declares_subtyping`
gates `wasm-execution`'s runtime dispatch between this nominal rule and
the engine's original pre-W33 plain-structural-equality behavior (see
below) — necessary because `type_subtyping.is_empty()` is not a reliable
"never uses `sub`" signal once the parser pushes a default entry for
every type regardless.

`wasm-validator::type_check::is_assignable` (`type_check.rs:194`) has
THREE nominal arms, all `ConcreteFuncRef`/`NonNullConcreteFuncRef`
pairs parametrized on flat type index, each calling
`module.func_type_is_nominal_subtype(i, j)`. **There is no equivalent arm
for `StructRef`/`NonNullStructRef`/`ArrayRef`/`NonNullArrayRef`** —
confirmed by direct read of the whole function — even though those
variants are already index-parametrized (`ValueType::StructRef(u32)`,
etc.) and `TypeSubtyping`/`nominal_subtype_chain` are already
kind-agnostic. This is a real, currently-open, narrower gap, adjacent to
but distinct from this spec's own canonical-equivalence scope: even
completing (3b) does nothing for struct/array `sub`-chain checking at a
`call`/`local.set`/`global.set` site until `is_assignable` grows the
analogous `StructRef`/`ArrayRef` arms. Noted here for whichever future
slice needs it (**Explicitly out of scope** below), because this spec's
own design (see "Design" §3) needs to touch `is_assignable` anyway and a
future session should not assume struct/array subtyping already works at
that call site just because `TypeSubtyping` covers all kinds.

`wasm-validator::type_check::check_type_subtyping`
(`type_check.rs:870`) — the module-level static-shape checker — iterates
`module.types` (the func-only Vec) and calls `func_is_structural_subtype`
unconditionally for every entry with a declared `supertype`. **This is
ALSO func-only, and — freshly confirmed by direct read, not previously
recorded anywhere — has no analogous width/depth/variance checker for
`StructType`/`ArrayType` bodies at all.** For a struct/array-kind flat
index, `module.types[i]` is the unused dummy `FuncType {params: vec![],
results: vec![]}` (per `TypeKind`'s own doc comment), so a declared `(sub
$parent (array ...))`/`(sub $parent (struct ...))` relationship is
checked against TWO EMPTY func signatures (trivially "compatible": arity
0, vacuous contravariance/covariance) instead of the real element/field
lists — a distinct, pre-existing correctness gap from real GC-proposal
struct/array structural subtyping (element covariance, mutable
invariance, struct width/depth), independent of canonical equivalence.
`check_type_subtyping_is_acyclic` (`type_check.rs:929`), by contrast, IS
already kind-agnostic (it only ever touches `supertype` indices, never a
type's body), so canonicalization can safely depend on it running first
regardless of what kind of type is involved.

### Runtime dispatch (`wasm-execution`)

`HostFunction` (`wasm-execution/src/lib.rs:1757`) — the trait every
callable (WASI shim, or a `CrossModuleFunction` standing in for another
WASM module's export) implements — has `func_type()`, `type_group_shape()
-> (u32, u32)` (default `(1, 0)`, a singleton group), and `is_final() ->
bool` (default `true`). `WasmExecutionContext` carries a flat
`type_subtyping: Vec<TypeSubtyping>` (`lib.rs:3579`), threaded in via
`set_type_subtyping`/`set_func_type_indices` (`lib.rs:12061`/`12079`) —
the SAME "parallel slice, not a whole `WasmModule`" pattern
`nominal_subtype_chain` was built for. `call_indirect_type_matches`
(`lib.rs:11273`) and `ref_matches_concrete_type` (`lib.rs:4472`) both
consult `any_declares_subtyping`/`nominal_subtype_chain` against this
slice to decide which of the two dispatch rules (legacy structural
equality vs. real nominal chain) applies.

### Cross-module linking (`wasm-runtime`, `wasm-conformance`)

`wasm-runtime`'s import resolution (`lib.rs:1401-1450`, the
`ImportTypeInfo::Function` arm) is the actual site of the W33 first
slice's "conservative guard": three ANDed checks — `host_func.func_type()
!= &ft` (plain structural `FuncType` equality), `host_func.
type_group_shape() != module.type_group_shape(*type_idx)` (rec-group
size+position match), `host_func.is_final() != module.type_subtyping_at
(*type_idx).is_final` (finality match) — each documented in-place as
"strictly additive, safe for every pre-existing import: can only ADD a
rejection, never remove one" precisely because it is NOT the real
algorithm, only a sound approximation of it. `wasm-conformance`'s
`CrossModuleFunction` (`wasm-conformance/src/lib.rs:300`) is the concrete
`HostFunction` impl standing in for "another WASM module's export" in
this crate's own conformance-test linking path (`register`/`(func
(import ...))` directives); it computes `group_shape`/`is_final` once at
`resolve_function` time and reports them through the trait.

`wasm-runtime::instantiate` (`lib.rs:1373`) takes a `&ValidatedModule`,
not a raw `&WasmModule` — `wasm-validator::validate()`
(`wasm-validator/src/lib.rs`) must run first and returns a
`ValidatedModule` (`lib.rs:127`, currently a bare newtype wrapping
`WasmModule`, nothing else). This ordering — validate, THEN
instantiate/link — is the existing seam this spec's design (§3 below)
plugs into.

## The real algorithm

### Authoritative sources

Two sources were read directly for this spec, matching the discipline
this campaign used for W24's `exnref` tag-byte fix (caught by reading the
reference interpreter's `decode.ml` directly, not a secondhand
description):

1. **The GC proposal's own formal prose**:
   `WebAssembly/gc`, `proposals/gc/MVP.md` (fetched fresh at time of
   writing; section headers `#### Type Definitions`, `#### Type
   Contexts`, `#### Auxiliary Definitions`, `#### Equivalence`, `####
   Subtyping` → `##### Type Indices`/`##### Composite Types`). This is
   the formal, human-readable specification of the algorithm — informal
   in the sense of being prose-with-inline-math rather than executable,
   but precise enough to derive an implementation from directly.
2. **The reference interpreter's actual OCaml code**, same repo:
   `interpreter/syntax/types.ml` (`roll_rec_type`, `unroll_rec_type`,
   `subst_of`, `subst_def_type`), `interpreter/valid/match.ml`
   (`match_def_type`, `match_heap_type`, `match_struct_type`, etc.), and
   `interpreter/valid/valid.ml` (`check_rec_type`, the actual call site).
   Reading the code resolved a real ambiguity the prose alone left open
   (see "One level of substitution, not deep recursion" below) — this is
   the same "spec text underspecifies, reference interpreter is the real
   source of truth" situation this task anticipated.

Both sources describe the SAME algorithm; citing both because the prose
gives the "why" and the code gives the exact, unambiguous "how."

### Grammar (MVP.md, "Type Definitions")

```
deftype  ::= rec <subtype>*
subtype  ::= sub final? <typeidx>* <comptype>
comptype ::= <functype> | <structtype> | <arraytype>
```

A single non-`rec`-wrapped `(type ...)` field is shorthand for a
`rec`-group of size 1 — this repo's own `TypeSubtyping::default()`
(`rec_group_size: 1, rec_group_position: 0`) already models exactly this
convention, and `wasm-wast-parser`'s phase-A/phase-B split (`module.rs`
lines 900-999) already treats every top-level `(type ...)` field as an
implicit singleton `rec` group for symbol-resolution purposes. The MVP
restricts a `subtype` to at most one declared supertype in practice
(`TypeSubtyping.supertype: Option<u32>`, already matching this).

### Rolling: turning absolute self/group references into de Bruijn indices

This is the mechanism this repo's own W33 spec called "course-of-values /
De-Bruijn-relative numbering." MVP.md's own formal name is **tying**
(`#### Auxiliary Definitions`, "Rolling a context type"):

> `tie($t) = tie_$t(<ctxtype>)` iff `$t = <ctxtype>`
> `tie_$t((rec <subtype>*).i) = (rec <subtype>*).i[$t':=rec.0, ...,
> $t'+N:=rec.N]` iff `$t' = $t-i` and `N = |<subtype>*|-1`

In plain terms: for a group starting at absolute type-section index `x`
with `N+1` members, replace every reference to an index in `[x, x+N]`
(this group's own range) with the relative marker `rec.i` (`i` = that
index minus `x`) — leaving references OUTSIDE the group (to an earlier,
already-defined group) as ordinary absolute indices.

The reference interpreter's `roll_rec_type` (`types.ml:252-259`) is the
literal, unambiguous implementation:

```ocaml
let roll_rec_type x (rt : rec_type) : rec_type =
  let RecT sts = rt in
  let y = Int32.add x (Lib.List32.length sts) in
  let s = function
    | StatX x' when x <= x' && x' < y -> VarHT (RecX (Int32.sub x' x))
    | var -> VarHT var
  in
  subst_rec_type s rt
```

`valid.ml`'s `check_rec_type` (line 189-192) is the actual call site —
called ONCE per `rec` group, in module-type-section declaration order,
immediately as each group is validated, with `x` = that group's own start
index (`Lib.List32.length c.types` at that point):

```ocaml
let check_rec_type (c : context) (rt : rec_type) at : context =
  let RecT sts = rt in
  let x = Lib.List32.length c.types in
  let c' = {c with types = c.types @ roll_def_types x rt} in
  ...
```

This confirms the algorithm is inherently **incremental and
group-ordered**: each group's rolled form is computed once, appended to a
running context, and later groups reference EARLIER groups'
already-rolled forms directly — never the other way around (WASM's own
validity rule already requires this: `type-rec.wast`'s own note, "a
reference to a type in a LATER, not-yet-started group still fails,"
matches MVP.md's `ok(x)` ordering constraint exactly, and this repo's
existing `check_type_subtyping_is_acyclic` already guarantees the
supertype half of this ordering holds before any canonicalization would
run).

### Equivalence: comparing tied forms, with one substitution to resolve cross-group references

MVP.md, `#### Equivalence`:

> two regular type indices are equivalent if they define equivalent tied
> context types: `$t == $t'` iff `tie($t) == tie($t')`
>
> two recursive type indices are equivalent if they project the same
> index: `rec.i == rec.i'` iff `i = i'`
>
> two recursive types are equivalent if they are equivalent pointwise:
> `(rec <subtype>*) == (rec <subtype'>*)` iff `(<subtype> ==
> <subtype'>)*`
>
> notably, two subtypes are equivalent if their structure is equivalent,
> they have equivalent supertypes, and their finality flag matches:
> `(sub final1? $t* <comptype>) == (sub final2? $t'* <comptype'>)` iff
> `<comptype> == <comptype'>` and `($t == $t')*` and `final1? = final2?`

And, crucially, MVP.md's own **Note 2**, which is the direct license for
the "compute once, compare cheaply" implementation strategy this spec
recommends:

> type equivalence checks can be implemented in constant-time by
> representing all types as trees in tied form and canonicalising them
> bottom-up in linear time upfront.

**One level of substitution, not deep recursion** — the detail the
prose leaves informal but the reference interpreter's code makes exact.
`match_def_type` (`match.ml:144-148`):

```ocaml
and match_def_type c dt1 dt2 =
  dt1 == dt2 ||  (* optimisation *)
  let s = subst_of c in subst_def_type s dt1 = subst_def_type s dt2 ||
  let SubT (_fin, hts1, _st) = unroll_def_type dt1 in
  List.exists (fun ht1 -> match_heap_type c ht1 (DefHT dt2)) hts1
```

`subst_of c` (`types.ml:150-152`) maps every remaining absolute reference
`StatX x` (necessarily to an EARLIER, already-fully-rolled group, since
`dt1`/`dt2` are themselves already-rolled — their OWN group's internal
refs are already `RecX`) to `DefHT (context[x])` — i.e., it EMBEDS that
earlier group's already-rolled `def_type` value WHOLESALE, one level, and
stops (`subst_heap_type`'s own `DefHT dt -> DefHT dt (* assume closed
*)` case does NOT recurse into `dt`). This is correct, and terminates in
one pass, precisely BECAUSE `dt` was already fully rolled and closed when
ITS OWN group was processed earlier — no further substitution is needed
inside it. `subst_def_type s dt1 = subst_def_type s dt2` then falls
through to OCaml's ordinary structural (not pointer) equality over the
resulting fully-closed, finite value trees.

So the real algorithm is exactly the incremental, bottom-up scheme this
repo's own W33 spec anticipated, made precise:

1. Process type-section groups **in declaration order**.
2. For each group at start index `x`, size `N+1`: build each member's
   comptype + declared-supertype-list body, replacing every reference
   into `[x, x+N]` with a relative `Rec(i)` marker (rolling/tying), and
   every reference to an index `< x` with a **wholesale embedded copy**
   of that EARLIER group's already-computed canonical form (no further
   substitution needed, by induction).
3. The result — one canonical, self-contained, finite value per group —
   is this group's canonical form. A type's own canonical identity is
   `(group's canonical form, its position within the group)`.
4. **Equivalence** of two type indices — within one module, or across two
   modules that share no numbering at all — is ordinary structural
   (deep) equality of their `(canonical form, position)` pairs. No shared
   context, no shared numbering, and no linking-time re-derivation is
   needed: each module's canonicalization is entirely self-contained.
5. **Subtyping** composes exactly as `match_def_type` shows: `$t <: $t'`
   iff canonically-equivalent (step 4) OR `$t` declares some supertype
   `$t''` (from its own `sub` list) with `$t'' <: $t'`, transitively —
   this is EXACTLY this repo's existing `nominal_subtype_chain` shape,
   with the base/reflexive case upgraded from raw index equality to
   canonical equivalence.

### Subtyping across type indices (MVP.md, confirming step 5)

> Type indices are subtypes if they either define equivalent types or a
> suitable (direct or indirect) subtype relation has been declared:
> `$t <: $t'` if `$t = <ctxtype>` and `$t' = <ctxtype'>` and `<ctxtype>
> == <ctxtype'>`, or `unroll($t) = sub final? $t1* $t'' $t2* comptype`
> and `$t'' <: $t'`.
> Note: This rule climbs the supertype hierarchy until an equivalent type
> has been found. Effectively, this means that subtyping is "nominal"
> modulo type canonicalisation.

That last line is the single most useful sentence in the whole document
for this repo's purposes: **the real algorithm IS this repo's existing
nominal `sub`-chain walk, plus canonical equivalence as the termination
condition instead of raw index equality.** W33's first three slices
already built the "nominal chain walk" half correctly; this spec is only
the other half.

### Composite-type subtyping (MVP.md, `##### Composite Types`) — already implemented, cited for completeness

Function contravariance/covariance, struct width/depth, array/field
covariance-if-immutable/invariance-if-mutable: this is the rule W33's
first slice already implemented in `func_is_structural_subtype`
(func-only; see "What already exists" above for the struct/array gap) —
not re-derived here, since it's orthogonal to canonicalization itself
(it governs whether a DECLARED `sub` relationship is even legal, not
whether two independently-declared groups are the SAME type).

### Worked example, tying the algorithm to this repo's own cited corpus cases

MVP.md's own worked example (slightly condensed):

```wat
(rec (type $t1 (struct (field i32 (ref $t2))))
     (type $t2 (struct (field i64 (ref $t1)))))
```
Tying `$t1` (group start `x`, `N=1`): `tie($t1) = (rec (struct (field i32
(ref rec.1))) (struct (field i64 (ref rec.0)))).0`. A SEPARATE,
differently-named, differently-indexed group `$u1`/`$u2` with the
identical shape ties to the byte-identical value, so `$t1 == $u1`
trivially once tied. This is **exactly** `type-equivalence.wast`'s own
"Isomorphic recursive types" module (lines 49-71, verified fresh against
the pinned SHA `28864811cf03bdbf880733786148feaba339582d`):

```wat
(rec (type $t0 (func (param i32 (ref $t1))))
     (type $t1 (func (param i32 (ref $t0)))))
(rec (type $t2 (func (param i32 (ref $t3))))
     (type $t3 (func (param i32 (ref $t2)))))
```

and this repo's own W33-spec-cited "3-cycle" (`type-subtyping.wast` lines
68-87, also re-verified fresh):

```wat
(rec
  (type $t1 (sub (func (param i32 (ref $t3)))))
  (type $t2 (sub $t1 (func (param i32 (ref $t2)))))
  (type $t3 (sub $t2 (func (param i32 (ref $t1)))))
)
(func $f1 (param $r (ref $t1)) (call $f1 (local.get $r)))
(func $f2 (param $r (ref $t2)) (call $f1 (local.get $r)) (call $f2 ...))
(func $f3 (param $r (ref $t3)) (call $f1 ...) (call $f2 ...) (call $f3 ...))
```

`$f3`'s body calls `$f1` with a `(ref $t3)` value where a `(ref $t1)`
argument is expected. `$t3 <: $t1` requires climbing `$t3 <: $t2 <: $t1`
per the declared `sub` chain — and per the real rule above, EACH hop's
own termination check (`$t2 == $t2`? `$t1 == $t1`?) is a canonical
equivalence check, not a raw-index shortcut, because the chain and the
group's own internal recursive references (`$t2`'s own param is `(ref
$t2)`, `$t3`'s is `(ref $t1)`) are themselves tangled through the SAME
`rec` group being canonicalized. This repo's existing `nominal_subtype_
chain`'s reflexive base case (`sub_idx == super_idx`) already handles the
in-module, same-index case of this correctly by construction (comparing
an index to itself); what's missing is the case two DIFFERENT indices —
whether in the same module (`type-rec.wast`'s "Static/Dynamic matching"
sections) or different modules with no shared numbering at all
(`type-equivalence.wast`, `type-subtyping.wast`'s "Linking" section) —
are canonically the SAME type despite the index mismatch.

## Design: where this plugs into this crate

### 1. Representation: `CanonicalType`, a self-contained, comparable value tree

Following the reference interpreter's own "embed the earlier group's
already-closed value wholesale" strategy (not the perf-oriented,
hash-consed/interned variant MVP.md's Note 2 alludes to as a possible
FUTURE optimization for large modules) — a fully-inlined owned tree is
the right choice for a first implementation: this crate's own corpus has
at most a handful of mutually-recursive types per group and at most a
few groups deep, so the "linear time upfront, constant time per compare"
promise is achievable trivially even without interning, by deriving
`PartialEq`/`Eq`/`Hash`/`Clone` on ordinary nested Rust enums and letting
structural (not pointer) equality do the work — exactly what OCaml's
polymorphic `=` does in `match_def_type`.

Proposed new types, in `wasm_types` (natural home: same crate as
`TypeSubtyping`/`TypeKind`, since canonicalization is a pure function of
data already fully modeled there):

```rust
/// A self-contained, De-Bruijn-tied value tree for one composite type
/// (or one `rec` group's worth of them) — comparable via ordinary
/// structural equality across TWO DIFFERENT `WasmModule`s with no shared
/// numbering, per the WasmGC proposal's own canonicalization algorithm.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalGroup {
    pub members: Vec<CanonicalSubtype>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalSubtype {
    pub is_final: bool,
    /// Tied the same way as the body: `Rec(i)` for an in-group
    /// supertype, `Outer(..)` (a fully embedded, already-canonical
    /// earlier group) otherwise. `None` for no declared supertype.
    pub supertype: Option<CanonicalHeapRef>,
    pub comp: CanonicalCompType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalCompType {
    Func(Vec<CanonicalValType>, Vec<CanonicalValType>),
    Struct(Vec<CanonicalFieldType>),
    Array(CanonicalFieldType),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalFieldType {
    pub storage: CanonicalStorageType, // mirrors StorageType, no indices
    pub mutable: bool,
}

/// Mirrors `ValueType`, but every concrete/self/group reference has been
/// resolved to a `CanonicalHeapRef` instead of a raw flat index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalValType { I32, I64, F32, F64, V128, Ref(bool /* nullable */, CanonicalHeapRef) }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalHeapRef {
    Abstract(AbstractHeapKind), // Any/Eq/I31/Struct/Array/Func/None/Extern/NoExtern/NoFunc
    /// A reference within the SAME group being tied — the De Bruijn part.
    Rec(u32),
    /// A reference to an EARLIER group, embedded wholesale (already
    /// fully tied/closed when it was computed) plus this type's own
    /// position within that group.
    Outer(Box<CanonicalGroup>, u32),
}
```

`WasmModule` gains one new derived field, computed once (see §2), NOT
parsed:

```rust
/// One [`CanonicalType`] per flat type-section index — `(this type's own
/// group's canonical form, its position within it)`. Computed once by
/// `wasm-validator::validate` (W34), after `check_type_subtyping_is_
/// acyclic` has confirmed the ordering canonicalization depends on.
/// Empty for any `WasmModule` that predates this field, exactly
/// `TypeSubtyping`'s own "missing means legacy default" contract — every
/// consumer here falls back to the pre-W34 approximate checks when this
/// is empty/shorter than `types`.
pub canonical_types: Vec<(Rc<CanonicalGroup>, u32)>,
```

(`Rc`, not an owned `CanonicalGroup`, so cloning this Vec into
`WasmExecutionContext` — the same pattern `type_subtyping.clone()`
already uses at `wasm-runtime/src/lib.rs:2310`-ish — is O(types), not
O(total tree size); comparison still needs `Rc`'s `PartialEq` to compare
CONTENTS, not pointers, which `derive(PartialEq)` on a type that merely
CONTAINS an `Rc<T: PartialEq>` already does correctly in Rust — `Rc`'s
own `PartialEq` impl delegates to `T`'s.)

### 2. Where it's computed: `wasm-validator::validate`, cached on `ValidatedModule`

The natural seam is `wasm-validator::validate()` (`wasm-validator/src/
lib.rs`), immediately after `check_type_subtyping_is_acyclic` succeeds —
canonicalization's own termination argument depends on the module's
`sub`-graph already being acyclic and (per MVP.md's `ok(x)` rule) every
cross-reference already pointing only at an earlier-or-same group, both
of which that existing check (or an existing bounds/ordering check
alongside it) already establishes. Concretely:

```rust
fn canonicalize_types(module: &WasmModule) -> Vec<(Rc<CanonicalGroup>, u32)> {
    // Walk `type_subtyping` to find each group's (start, size); process
    // groups in flat-index order (== declaration order, since a `rec`
    // group's members always occupy a contiguous index range). For each
    // group: build every member's `CanonicalSubtype` by resolving each
    // `ValueType`'s reference — `< group_start` embeds the group at that
    // earlier index's ALREADY-COMPUTED `Rc<CanonicalGroup>` (already in
    // `canonical` below, by induction); `>= group_start && < group_end`
    // becomes `Rec(idx - group_start)`; anything else is an abstract
    // heap type, unchanged.
}
```

`ValidatedModule` (`wasm-validator/src/lib.rs:127`, currently a bare
`{ module: WasmModule }` newtype) is extended with the computed
`canonical_types` alongside `module` — a minimal, non-disruptive
extension of an already-existing "proof of validation" checkpoint type,
not a new architectural concept. This directly answers the concern the
task raised about needing lazy computation during cross-module linking:
**it does not** — canonicalization is a pure function of one module's
own, already-fully-parsed type section, is entirely self-contained (per
the algorithm's own design — no linking partner's types are ever
consulted), and `ValidatedModule` already exists as exactly the
"per-module, done once before instantiation/linking" checkpoint this
needs. `wasm-runtime::instantiate` already requires a `&ValidatedModule`,
never a raw `&WasmModule` (`lib.rs:1373`), so every consumer that needs
`canonical_types` already goes through the one place it would be
computed.

### 3. Wiring into within-module checks

- **`wasm-validator::type_check::is_assignable`** (`type_check.rs:194`):
  generalize the three existing `ConcreteFuncRef`/`NonNullConcreteFuncRef`
  nominal arms' termination — today `module.func_type_is_nominal_subtype
  (i, j)` walks the `sub` chain with a raw `i == j` base case; the base
  case becomes `module.canonically_equivalent(i, j) ||` (a new
  `WasmModule` method comparing `canonical_types[i]`/`canonical_types
  [j]`), keeping the existing chain-walk for the "declared, not just
  canonically equal" half. This is a small, mechanical change to an
  ALREADY-nominal-aware function — not a new capability grafted on. Per
  the adjacent gap noted in "What already exists," this is also the
  natural point to add the missing `StructRef`/`ArrayRef` arms (using the
  exact same generalized `sub_idx`/`super_idx` termination), though that
  addition is separable and could ship independently first.
- **`nominal_subtype_chain`** (`wasm-types/src/lib.rs:1632`): the
  free-function version needs the SAME reflexive-base-case upgrade, but
  it only has a `&[TypeSubtyping]` slice today, not `canonical_types` —
  it needs a second parallel slice parameter (or becomes a method taking
  both slices). This is the function `wasm-execution`'s runtime dispatch
  calls directly (see below), so this change ripples there too.
- **`wasm-execution`'s runtime dispatch** (`call_indirect_type_matches`,
  `ref_matches_concrete_type`): needs `WasmExecutionContext` to carry a
  new parallel `canonical_types: Vec<(Rc<CanonicalGroup>, u32)>` field,
  threaded in via a new `set_canonical_types` setter mirroring
  `set_type_subtyping`'s exact shape. This is mechanical, but it is a
  THREE-crate change for this one slice — `wasm-runtime` (populate the
  setter from `ValidatedModule`/`WasmInstance`), `wasm-execution` (carry
  the field, use it in the two dispatch functions), and
  `wasm-conformance`'s `CrossModuleFunction`/`HostFunction` impls stay
  untouched for THIS slice specifically (within-module dispatch doesn't
  need cross-module `HostFunction` plumbing) — worth flagging plainly so
  a future session doesn't underestimate this slice's footprint as
  "just `wasm-validator`."
- **Corpus proof points** (verified fresh against the pinned SHA): all of
  `type-rec.wast`'s "Static matching of recursive types" (lines 69-132)
  and "Dynamic matching of recursive function types" (lines 165-192) —
  same-module `global`/`call_indirect` checks against two separately-`rec`
  -declared, structurally-identical groups; `type-equivalence.wast`'s
  "Recursive types"/"Isomorphic recursive types" modules under both
  "Syntactic types" (lines 28-71) and "Semantic types (run time)" (lines
  134-188); `type-subtyping.wast`'s "Subsumption" 3-cycle and 4-member
  cases (lines 68-113).

### 4. Wiring into cross-module linking

- **`HostFunction` trait** (`wasm-execution/src/lib.rs:1757`) gains a
  fourth accessor, e.g. `fn canonical_type(&self) -> Option<(Rc<
  CanonicalGroup>, u32)>` (default `None`, meaning "no canonical
  identity available — fall back to the existing `type_group_shape`/
  `is_final` conservative guard," correct for every existing WASI-shim
  `HostFunction` impl, none of which have or need a `CanonicalGroup`).
- **`wasm-conformance`'s `CrossModuleFunction`** (`lib.rs:300`) is the
  ONE real implementor that would return `Some(..)`: it already computes
  `group_shape`/`is_final` once at `resolve_function` time from the
  exporting module's `ValidatedModule`/`WasmInstance`; it gains a third
  precomputed field, the exporting function's own `canonical_types[idx]`
  entry, cloned (cheap, `Rc`-backed) at the same resolution point.
- **`wasm-runtime`'s import-compatibility check**
  (`lib.rs:1401-1450`): the real fix REPLACES (not just supplements) the
  three-part conservative guard. When both sides report `Some(..)`
  canonical types, compare them directly (`Rc<CanonicalGroup>`/position
  pair equality — no shared numbering needed, per the algorithm's own
  design) — this SUBSUMES the `type_group_shape`/`is_final` checks
  entirely (both are already folded into `CanonicalSubtype`'s own
  `is_final`/`supertype` fields), and additionally accepts the cases the
  conservative guard wrongly rejected (isomorphic-but-differently-shaped
  groups) while STILL rejecting everything it correctly rejected before
  (the guard was already proven sound, only incomplete). When either side
  reports `None` (a non-`CrossModuleFunction` host import), fall back to
  the existing three-part guard unchanged — this keeps every WASI-shim
  import byte-for-byte as before.
- **Corpus proof points**: `type-equivalence.wast`'s full "Semantic types
  (link time)" section (lines 191-324, all 6 `register`/import pairs,
  including the "Isomorphic recursive types" 4-module block at lines
  244-324 — the one this file's own doc header calls out as testing
  cross-group isomorphism with NO shared numbering at all);
  `type-subtyping.wast`'s "Linking" section in full (lines 538-774: `M1`
  onward, `M3` at 620-630 — cross-module isomorphic-rec-group import,
  currently gated by `wasm-conformance`'s own already-verified
  cross-module test harness — `M5` at 652-666, the "one field reference
  wrong" negative case, and the `M10`/`M11` pair at 746-774, the
  topology-mismatch case the conservative guard cannot reach by
  construction); `type-rec.wast`'s "Link-time matching of recursive
  function types" (lines 135-162).

## Recommended slice decomposition

Refined from the task's own suggested split by the investigation above —
narrower first slice than "all non-recursive types" (self-reference
inside a singleton group is NOT trivial and needs the SAME `Rec`
machinery as a real multi-member group; only the CROSS-member numbering
is what's actually deferred to slice 2), and slice 3 now correctly
scoped as a three-crate change rather than a `wasm-validator`-only one:

1. **`CanonicalGroup`/`CanonicalType` representation + canonicalization
   for SELF-REFERENCING SINGLETON groups (`rec_group_size == 1`) and
   groups with NO internal cross-references at all.** Proves: the
   `CanonicalGroup`/`CanonicalHeapRef` representation's `Eq`/`Hash`
   correctness, the `Rec(0)` self-reference case (the ONLY case a
   singleton group can even exercise), and — crucially — the
   cross-module comparability property (two independently-canonicalized
   `WasmModule`s' outputs compare correctly with no shared context) using
   the SIMPLEST possible non-trivial case, before attempting real
   multi-member De Bruijn numbering. Test against: `type-rec.wast` lines
   1-19 (`(rec (type $g (func (param (ref $g)) (result (ref $g)))))`,
   line 14 — self-reference; line 4's flat, non-`rec`-wrapped
   self-referencing type, this repo's own implicit-singleton-group
   convention); `type-equivalence.wast`'s "Simple types"/"Indirect types"
   modules (lines 1-25, 89-131, 191-229) — non-recursive, cross-index-
   but-non-cyclic-referencing types, including the positive case that
   `$t1`/`$t2` at lines 6-7 (identical bodies, different PARAMETER NAMES
   only) canonicalize identically, a good "does the representation throw
   away irrelevant syntax" proof point independent of recursion entirely.

2. **Multi-member `rec`-group canonicalization — the real De Bruijn
   numbering.** Extend to `rec_group_size > 1`: intra-group references
   become `Rec(i)` relative to the group's OWN start (not the module's
   absolute index), inter-group references embed the earlier group's
   already-computed `Rc<CanonicalGroup>` wholesale. Test against:
   `type-canon.wast` in full (both modules — a pure "does this parse and
   canonicalize without panicking/looping" proof, since the file makes NO
   assertions at all — confirmed by direct read); `type-rec.wast`'s
   remaining "Syntax and Scoping" cases (lines 5-8, the 2-member
   `$h`/`$k` mutual pair at 15-18); `type-equivalence.wast`'s "Recursive
   types" and "Isomorphic recursive types" modules under "Syntactic
   types" (lines 28-71) — checked via a NEW, temporary, canonicalization-
   only unit test comparing `canonical_types` entries directly (this
   slice does not yet wire anything into `is_assignable`/`call_indirect`,
   so the corpus's own `assert_return`/`assert_invalid` directives for
   these modules won't move yet — that's slice 3).

3. **Wire canonical equivalence into WITHIN-MODULE checks** — across
   THREE crates, per the design's own finding above: `wasm-validator`
   (`is_assignable`'s reflexive-base-case upgrade, `nominal_subtype_
   chain`'s analogous upgrade), `wasm-runtime` (a new `set_canonical_
   types` call alongside the existing `set_type_subtyping` one),
   `wasm-execution` (`WasmExecutionContext`'s new field, `call_indirect_
   type_matches`/`ref_matches_concrete_type`'s upgraded termination
   check). Test against: `type-rec.wast`'s "Static matching of recursive
   types" (69-132) and "Dynamic matching of recursive function types"
   (165-192) — BOTH now expected to move, correcting the third addendum's
   "same-module cases don't need (3b)" guess, per the fourth addendum's
   own confirmed finding; `type-equivalence.wast`'s "Semantic types (run
   time)" section (85-188); `type-subtyping.wast`'s "Subsumption" section
   (68-113), the 3-cycle and 4-member cases specifically, since these are
   the cases whose subtype-chain termination genuinely depends on
   canonical equivalence, not just reflexive index equality.

4. **Wire into CROSS-MODULE linking** — `HostFunction`'s new
   `canonical_type()` accessor, `CrossModuleFunction`'s implementation of
   it, `wasm-runtime`'s import-check replacing (not just supplementing)
   its three-part conservative guard when both sides report a real
   canonical type. Test against: `type-equivalence.wast`'s "Semantic
   types (link time)" section in full (191-324, all 6 pairs);
   `type-subtyping.wast`'s "Linking" section in full (538-774, including
   `M3`/`M5`/`M10`/`M11`); `type-rec.wast`'s "Link-time matching of
   recursive function types" (135-162).

Each slice's own tests are gated on the previous slice's representation
existing, but NOT on the previous slice's WIRING being complete — slice 2
can be fully verified via direct unit tests on `canonical_types` output
before slice 3 wires anything into a validator/execution call site, the
same "verify the narrow piece before attempting the wiring" discipline
W33's own "Recommended scope" used.

## Explicitly out of scope for this spec

- **Struct/array nominal-subtype arms in `is_assignable`** (freshly found
  above): `is_assignable` has zero `StructRef`/`ArrayRef` arms at all,
  canonical or nominal. Slice 3 above touches the SAME function and could
  add these as a natural side effect, but it is a separable, smaller,
  independently-shippable piece — not gated on canonicalization existing
  (a plain nominal `sub`-chain check for struct/array would work today,
  using the exact same `func_type_is_nominal_subtype`-shaped machinery,
  entirely independent of this spec).
- **Struct/array structural-subtype checking in `check_type_subtyping`**
  (freshly found above): `func_is_structural_subtype` is the only
  structural-subtype checker that exists; a declared `(sub $parent
  (struct/array ...))` relationship is currently checked against two
  EMPTY dummy `FuncType`s instead of the real field/element lists. This
  predates W34 entirely (it's a W33-first-slice-era gap that struct/array
  parsing, shipped in the fourth slice, made newly REACHABLE without
  fixing), is orthogonal to canonical equivalence (it's about whether a
  DECLARED `sub` relationship is legal, not whether two independent
  groups are the SAME type), and needs its own investigation into exactly
  which `type-subtyping.wast` "Invalid subtyping definitions" directives
  it affects (not fully traced here — flagged, not scoped).
- **Interning/hash-consing for performance.** MVP.md's Note 2 mentions
  this as a possible optimization for large modules; this repo's own
  corpus modules are all small enough that the straightforward
  fully-inlined `Rc<CanonicalGroup>` tree (§1 above) meets the "linear
  time upfront, cheap compare" goal without it. Revisit only if a real
  performance problem is measured, not speculatively.
- **`array.new_data`/`array.new_elem`/`array.copy`/`array.fill`** and the
  no-instruction-level-real-result-type-checker gap (both already tracked
  in W33's fourth addendum) — unrelated to canonical equivalence.
- **Global const-expr type-checking** and **per-local
  definite-initialization tracking** — already closed (W33 second slice)
  or already tracked elsewhere (W32's second addendum); unrelated here.
- **Extending `ValidatedModule`'s public API surface beyond
  `canonical_types`** — e.g. exposing canonicalization as a public,
  reusable standalone function outside `wasm-validator`/`wasm_types` for
  other potential embedders. Not needed for anything this spec's own
  scope requires; if a future consumer needs it, it's a trivial visibility
  change, not a design question.

## Verification plan (for whatever session implements this)

- Build slice 1 first and verify `CanonicalGroup`/`CanonicalHeapRef`'s
  own `Eq`/`Hash` derivation directly via targeted unit tests (two
  independently-constructed `WasmModule`s, canonicalized separately,
  compared) before touching any corpus file — this is the "prove the
  representation is even comparable across modules with no shared
  context" step the whole algorithm's cross-module promise depends on.
- Build slice 2 next; verify against `type-canon.wast` (parses and
  canonicalizes without panicking — it has no assertions to check
  against, so this is a smoke test, not a correctness proof) AND a set of
  hand-written unit tests asserting specific `type-equivalence.wast`
  module pairs' `canonical_types` entries ARE equal (the isomorphic
  cases) or ARE NOT equal (deliberately non-isomorphic pairs, to catch a
  canonicalizer that's too permissive) — do this BEFORE wiring anything
  into `is_assignable`/`call_indirect`, so a representation bug is caught
  at the narrowest possible layer.
- Build slice 3; re-run `type-rec.wast`/`type-equivalence.wast`/
  `type-subtyping.wast`'s own baseline and diff programmatically. Expect
  `type-equivalence.wast`'s "Syntactic"/"Semantic types (run time)"
  sections and `type-rec.wast`'s "Static"/"Dynamic matching" sections to
  move; expect NO other file's tally to change (the `any_declares_
  subtyping`-style gating this repo already uses for `wasm-execution`'s
  dispatch rule should be reused/extended here too, so a module that
  never uses `rec`/`sub` at all is provably unaffected).
- Build slice 4 last; re-run `type-equivalence.wast`'s "link time"
  section and `type-subtyping.wast`'s "Linking" section specifically —
  expect the 2 remaining `M10`/`M11` `assert_unlinkable` fails (present
  since the first slice) to finally resolve, and expect NO previously
  passing `assert_unlinkable` case to flip to a false accept (the
  conservative guard's own soundness proof — "can only add a rejection,
  never remove one" — must still hold in spirit for the new check: verify
  by keeping the OLD three-part guard as a fallback-only path, not
  deleting it, so a `None`-canonical-type host import is provably
  unaffected).
- Re-run the full conformance baseline
  (`cargo run --bin wasm_conformance_report -p wasm-conformance --
  --write-baseline`) and diff programmatically after EVERY slice, not
  just at the end — this is the same discipline every W33 addendum used,
  and this epic is at least as easy to mis-attribute a regression within
  if diffed only at the end.
- Run `cargo test --workspace` (or the equivalent per-crate command) after
  each slice, not just the conformance baseline — `wasm_types`,
  `wasm-validator`, `wasm-execution`, and `wasm-runtime` all have
  extensive existing unit-test suites for the pre-W34 nominal-subtyping
  machinery (`nominal_subtype_chain`'s own cycle/hop-bound tests,
  `type_group_shape`'s own tests, `wasm-runtime`'s own `incompatible
  import type` assertions) that a careless reflexive-base-case change
  could silently break.

## Addendum — first slice shipped (singleton-group canonicalization)

Re-verified this spec's own citations fresh against the pinned SHA
(`28864811cf03bdbf880733786148feaba339582d`) before writing any code:
`type-rec.wast`, `type-equivalence.wast`, `type-subtyping.wast`, and
`type-canon.wast` all matched this document's line-number claims exactly
(line 4's flat self-reference and line 14's explicit-singleton
self-reference in `type-rec.wast`; `type-equivalence.wast`'s "Simple
types"/"Indirect types"/"Recursive types"/"Isomorphic recursive types"
modules at the cited lines; `type-canon.wast`'s two modules, both
genuinely multi-member-only with zero assertions). The crate-version
grounding ("What already exists," `wasm-execution` 0.9.82) had already
drifted to 0.9.83 by the time this slice started (one unrelated patch
landed in between) — re-checked directly: `HostFunction`'s definition is
still at the exact cited line 1757, so nothing in the design section
needed correcting for that drift. `ValidatedModule`'s `module` field was
re-confirmed private with `validate()` as the sole constructor (the
W33-era security fix) — exactly the property this slice's own
`canonical_types` caching leans on, confirming §2's "natural,
non-disruptive caching point" claim.

**What shipped** (`wasm_types` 0.1.17 → 0.1.18, `wasm-validator` 0.2.79 →
0.2.80):

- `wasm_types`: `CanonicalGroup`, `CanonicalSubtype`, `CanonicalCompType`,
  `CanonicalFieldType`, `CanonicalStorageType`, `CanonicalValType`,
  `CanonicalHeapRef`, `AbstractHeapKind`, and the `canonicalize_types(
  &WasmModule) -> Vec<Option<(Rc<CanonicalGroup>, u32)>>` free function —
  exactly the representation this document's own "Design §1" sketched,
  scoped to `rec_group_size == 1` groups only (both the implicit,
  non-`rec`-wrapped kind and an explicit `(rec (type ...))` with exactly
  one member). A self-reference ties to `Rec(0)`; a reference to an
  earlier singleton group embeds that group's already-computed form via
  `Outer`; a reference into an unsupported `rec_group_size > 1` group (or
  any other unresolvable index) makes the REFERRING type's own canonical
  form `None` too, never a partial or wrong tree. `canonicalize_types`
  itself contains no recursion of any kind — it walks flat indices in
  increasing order and only ever reads already-computed entries — so a
  cyclic or self-referential type structure, even from a hand-built
  `WasmModule` that never went through validation, cannot make it loop,
  panic, or overflow the stack; the worst case is an honest `None`.
- `wasm-validator`: `ValidatedModule` gains a private `canonical_types`
  field (computed in `validate()` right after Check 11's
  `check_type_subtyping_is_acyclic` succeeds) and two new public methods,
  `canonical_type_at` and `canonically_equivalent`. Nothing wires this
  into any actual validation DECISION yet — `is_assignable`,
  `check_type_subtyping`, `nominal_subtype_chain`, and every
  `wasm-execution`/`wasm-runtime` dispatch/import-check site are
  byte-for-byte unchanged. That wiring is slice 3 (and slice 4 for
  cross-module linking), both still gated on slice 2's real multi-member
  De Bruijn numbering landing first, per this document's own decomposition.
- 22 new unit tests across the two crates: the `Rec(0)` self-reference
  case (both implicit and explicit-singleton spellings, proven to tie
  identically); cross-module comparability with NO shared numbering (two
  independently-built `WasmModule`s, isomorphic shape at different flat
  indices, proven equal — both directly via `canonicalize_types` and
  end-to-end through `validate()`); a genuine shape mismatch proven NOT
  equal; parameter-name-irrelevance (`type-equivalence.wast` lines 6-7's
  own point); chained (non-self-referencing) singleton `Outer` embedding
  across modules; multi-member groups correctly producing `None`
  everywhere including through a referring singleton; finality/declared-
  supertype as part of canonical identity; struct/array bodies (not just
  func ones); every abstract heap-type variant; and two defensive/security
  cases (an out-of-range supertype index, and a self-referential declared
  supertype) proven to produce a safe `None`/`Rec(0)` rather than a panic
  or a loop.

**One correction to this document's own design section, found by
re-verification, not by assumption**: §1's `AbstractHeapKind` sketch
listed only the ten kinds the WasmGC proposal's own lattice names
(`Any`/`Eq`/`I31`/`Struct`/`Array`/`Func`/`None`/`Extern`/`NoExtern`/
`NoFunc`). Re-checking it against this crate's ACTUAL `ValueType` (not
the design sketch's own memory of it) found two more variants already
shipped and needing somewhere to tie to: `ValueType::Exnref`/`ValueType::
NullExnref` (W24, the separate exceptions proposal — not part of WasmGC's
own MVP.md lattice at all, but real, existing, and reachable through any
func/struct/array field). Added `Exn`/`NoExn` to `AbstractHeapKind` to
close this gap; every one of this crate's 21 `ValueType` variants now has
an exhaustive, panic-free mapping (`every_abstract_heap_type_
canonicalizes_deterministically` covers all ten abstract, non-index-
carrying variants directly; the concrete/index-carrying ones are covered
by the `Rec`/`Outer` tests above).

**One deliberate deviation from the design sketch's literal types,
recorded and justified, not silently substituted**: `CanonicalHeapRef::
Outer` uses `Rc<CanonicalGroup>` where §1's sketch wrote `Box<
CanonicalGroup>`. A `Box` would deep-clone the entire referenced group's
tree at every embed site, which — unlike the sketch's own reasoning for
why a fully-inlined tree (rather than interning) is fine for THIS crate's
small corpus — would still duplicate a shared subtree once per reference
site within a single canonicalization pass (`type-rec.wast`'s own
"Static matching" module references `$f1`/`$f2` several times each from
sibling groups). `Rc` shares the one already-computed allocation instead;
`derive(PartialEq, Eq, Hash)` on a type containing an `Rc<T>` already
compares/hashes through to `T`'s contents, never the pointer, so this
costs nothing for the cross-module-comparability property the whole
mechanism exists for. `wasm-validator::ValidatedModule`'s own top-level
`canonical_types` cache already used `Rc` for the identical reason
(cheap-to-clone caching); this makes the choice consistent at every level
of the tree, not just the outermost one.

**Corpus impact: none, confirmed by a full 257-file baseline diff, not
just asserted.** `--write-baseline` was re-run and diffed programmatically
against the pre-slice baseline: every one of the 257 files' `module`/
`register`/`action`/`assert_return`/`assert_trap`/`assert_exhaustion`/
`assert_invalid`/`assert_malformed`/`assert_unlinkable`/`assert_exception`
tallies is byte-for-byte identical, including all four of this slice's
own cited files (`type-rec.wast`, `type-equivalence.wast`,
`type-subtyping.wast`, `type-canon.wast`). This matches this document's
own honest prediction in "Recommended slice decomposition" #1 ("this
slice does not yet wire anything into `is_assignable`/`call_indirect`, so
the corpus's own `assert_return`/`assert_invalid` directives for these
modules won't move yet — that's slice 3") — confirmed by measurement, not
just re-stated: nothing in this slice is reachable from any validation or
execution DECISION path, only from the two new public accessor methods
and this slice's own unit tests, so a zero-movement diff is exactly the
expected, correctly-predicted outcome, not a sign the slice did nothing
real.

**Security review finding, fixed before push**: a dedicated security-review
sub-agent, briefed specifically on this document's own two named concerns
(cyclic/self-referential-structure recursion, and whether `ValidatedModule`
caching could be bypassed), confirmed the SECOND concern was already fully
closed by `ValidatedModule::module`'s pre-existing W33-era privacy (no
construction path exists outside `validate()`; no staleness is possible
since nothing ever mutates `module` after construction). On the FIRST
concern, it confirmed `canonicalize_types` and everything it calls
genuinely never recurses while BUILDING a tree (verified by reading the
whole call graph) — but it went further than the question as posed and
empirically built a throwaway reproduction against this exact code,
finding that a long, entirely acyclic CHAIN of singleton groups (each
referencing only the type immediately before it — ordinary, not
pathological, WASM shape) builds a genuinely nested `Outer`-embedding tree
that the compiler-DERIVED `Drop`/`PartialEq`/`Hash` implementations (all
necessary for this mechanism's own correctness — structural, not pointer,
comparison is the entire point) walk RECURSIVELY, reliably crashing the
process via stack overflow at tens of thousands of chained links — a
small, realistic module size. Fixed by capping how deep `canonicalize_
types` will ever let an `Outer` chain nest (`MAX_CANONICAL_OUTER_DEPTH =
1,000`, mirroring this crate's own pre-existing `MAX_SUBTYPE_CHAIN_HOPS`
convention exactly) at `resolve_heap_index`, the one place new depth is
introduced — this closes the `Drop` crash the review reproduced AND the
parallel (not separately reproduced, but architecturally identical)
`PartialEq`/`Hash` recursion-depth risk in the same stroke, since all
three traversals share the same root cause (the depth of the value tree
itself) rather than needing three separate hand-rewritten iterative
implementations. A new regression test (`outer_embedding_depth_is_capped_
and_a_long_chain_does_not_crash`) builds a chain past the cap and confirms
both that in-bounds entries still canonicalize normally and that the
whole result drops cleanly at the end of the test. This is a real,
permanent limitation worth flagging for whoever builds slice 2/3: an
adversarial module with an extremely long reference chain will now
canonicalize to `None` past 1,000 links rather than being treated as
equivalent to anything — a conservative, safe direction (a missed
optimization opportunity, never a false accept) consistent with every
other "unresolvable falls back to `None`" case this slice already
established.

**Slice 2 plan: confirmed unchanged.** Nothing this slice's investigation
found requires revising the "multi-member `rec`-group canonicalization —
the real De Bruijn numbering" scope or its `type-canon.wast`/`type-rec.
wast`/`type-equivalence.wast` test-target list. One implementation note
for whoever picks up slice 2: the `total_type_count`/`comp_type_at`
helpers and the `resolve_heap_index`/`canonicalize_value_type`/
`canonicalize_field_type`/`canonicalize_comp_type` functions this slice
added in `wasm_types::lib` are already written generically enough (they
take a `self_idx`/group-relative reference-resolution shape, not a
singleton-specific one) that slice 2 should be able to reuse them
directly for each multi-member group's per-member body construction —
the only genuinely NEW logic slice 2 needs is the group-relative `Rec(i)`
numbering itself (mapping `[group_start, group_start+size)` to `0..size`,
not just the singleton case's trivial `self_idx == group_start`), and the
top-level `canonicalize_types` loop's own "process one flat index at a
time" structure will need to become "process one GROUP (a contiguous
range of flat indices) at a time" instead.
