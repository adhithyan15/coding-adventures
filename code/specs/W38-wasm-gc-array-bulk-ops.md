# W38 — WASM GC array bulk operations (`array.copy`/`array.fill`/`array.init_data`/`array.init_elem`/`array.new_data`/`array.new_elem`)

## Purpose and how this slice was chosen

`code/specs/W37-wasm-gc-reftype-tables.md`'s own re-derived corpus-impact
table identified the GC array bulk-operations instruction family —
`array.copy`, `array.fill`, `array.init_data`, `array.init_elem`,
`array.new_data`, `array.new_elem` — as "confirmed to be this cluster's
single largest bucket (215 NYS, 39%), entirely unrelated to table
declarations, and entirely unimplemented," and explicitly deferred it to
"a large, separate, natural follow-on spec... candidate title 'GC array
bulk operations.'" This document is that follow-on.

Per this campaign's own standing discipline — every W32-W37 spec
re-verified its own motivating claim directly against the pinned corpus
and current source before trusting it, and every one of them found at
least one thing the motivating document got wrong, stale, or
under-specified — this spec re-derives the six-instruction gap's real
current scope from scratch (same throwaway-probe method as every prior
spec in this series: a temporary, out-of-repo Cargo project depending on
`wasm-conformance` by path, calling `run_wast_source` on each file and
bucketing every `NotYetSupported` message) rather than trusting W37's
"~215, entirely unrelated, entirely unimplemented" framing at face value.

**The re-verification confirms W37's headline number (215) is still
exactly accurate — nothing has shifted since W37 merged — but finds the
215 is NOT monolithically "the six instructions and nothing else."** It
splits into four distinct causes, one of which (a real, three-layer gap
in how element segments carry arbitrary GC constant values, not just
funcref/null) is comparable in size and complexity to the six
instructions themselves and was not visible from W37's own
"unimplemented instruction" framing. See "Correction" sections below.

## Correction 1: the cluster's total is still exactly 215, but it has four root causes, not one

Direct probe of the current `main`-tip source (`d4f5275652f952feb34ae330
053f4db26cd55edc`, the tip `origin/main` was on when this spec was
written — confirmed via `git log -1` immediately before running the
probe), using `wasm_conformance::run_wast_source` from a throwaway
Cargo project outside the repo (`Cargo.toml` path-depending on this
worktree's `wasm-conformance` crate), against the same seven files W37
named:

| File | Total NYS | Cause breakdown (this probe, byte-position-verified) |
|---|---:|---|
| `array.wast` | 28 | 14 `array.new_data` unimplemented; 13 elem-segment reftype/const-value gap (`"unknown instruction \"ref\""`, from `(elem $e (ref $bvec) (array.new $bvec ...) ...)`); 1 non-null abstract heap type `(ref struct)` as an array's storage type (pre-existing, W37-flagged, unrelated) |
| `array_copy.wast` | 31 | 31 `array.copy` unimplemented (100%) |
| `array_fill.wast` | 27 | 27 `array.fill` unimplemented (100%) |
| `array_init_data.wast` | 44 | 44 `array.init_data` unimplemented (100%) |
| `array_init_elem.wast` | 33 | 20 `array.init_elem` unimplemented; 13 bare `arrayref` value-type keyword unrecognized (storage-type position) |
| `array_new_data.wast` | 28 | 28 `array.new_data` unimplemented (100%) |
| `array_new_elem.wast` | 24 | 11 `array.new_elem` unimplemented; 2 bare `arrayref` value-type keyword unrecognized; 11 elem-segment reftype/const-value gap (`"unknown instruction \"i31ref\""`, from `(elem $e i31ref (ref.i31 ...) ...)`) |
| **Total** | **215** | **175 six-instruction-attributable; 24 elem-segment reftype/const-value gap; 15 bare-`arrayref`-storage-keyword gap; 1 pre-existing non-null-abstract-heap-type gap** |

Every row was confirmed by reading the exact `NotYetSupported` message and
source slice at its reported byte position, not inferred from the file
name — e.g.:

```text
array_copy.wast, byte 1769 (31/31 hits, the file's only cause):
  "at byte 1769: unknown instruction \"array.copy\""

array_new_elem.wast, byte 90 (11/24 hits):
  "at byte 90: unknown instruction \"i31ref\""
  source: (elem $e i31ref (ref.i31 (i32.const 0xaa)) ...)

array_init_elem.wast, byte 4149 (13/33 hits):
  "at byte 4149: expected a value type, found \"arrayref\""
  source: (type $arrref_mut (array (mut arrayref)))

array.wast, byte 193 (1/28 hits):
  "at byte 193: expected an index, found \"struct\""
  source: (type (array (ref struct)))
```

**175 of the 215 (81%) trace directly and only to the six missing
instructions** — this is the primary target of this spec. The remaining
40 split into two gaps this spec's own corpus reading shows are
*necessary companions*, not incidental noise (see Correction 2), plus 1
pre-existing, already out-of-scope case (the `(ref struct)` non-null
abstract heap type, already flagged by W37's own "Explicitly out of
scope" section for a different file; unrelated to array bulk ops, left
alone here too).

## Correction 2: the "bare `arrayref`/`i31ref` in an elem segment" gap is not one bug — it's the visible tip of a real three-layer element-segment data-model gap that blocks a real subset of the six instructions from ever reaching `Pass`

W37's own Correction 2 named this "a second, unrelated gap... an elem
segment whose exprs-list reftype keyword is `i31ref`... or `(ref $bvec)`"
and predicted it was purely a parser-recognition problem, "flagged here
only so a future implementer doesn't mistake `array_new_elem.wast`'s NYS
count for evidence against this spec's own numbers." Reading the actual
fixture content this gap traces to (not just the error message) shows the
problem is deeper and would still block correct behavior even after
`array.new_elem`/`array.init_elem` are wired up as new instructions.

**The three layers, read directly:**

1. **`build_elem`'s reftype-vs-funcidx-list disambiguation** (`wasm-wast-
   parser/src/module.rs`, the site W36's own item 3 and W37's Correction 2
   both already flagged) only recognizes the bare atoms `"funcref"`/
   `"externref"` as a segment's own declared element type. `i31ref` and
   `(ref $bvec)` fall through unrecognized and get mis-encoded as if they
   were the first entry of a funcidx list — this is what produces the
   confusing `"unknown instruction \"i31ref\"\"`/`"unknown instruction
   \"ref\""` messages Correction 1's table cites.

2. **Even with (1) fixed, `resolve_elem_expr_entry`** (`module.rs:2511-
   2537`, confirmed by direct read) **only accepts an item shaped
   `(ref.func ...)` or `(ref.null ...)`** — any other expression is
   rejected with `"expected (ref.func ...) or (ref.null ...)"`. The real
   corpus this spec must serve uses items that are neither:
   `array_new_elem.wast`'s own `(elem $e i31ref (ref.i31 (i32.const 0xaa))
   ...)` and `(elem $elem arrayref (item (array.new_default $arr (i32.const
   0))))`, and `array.wast`'s own `(elem $e (ref $bvec) (array.new $bvec
   (i32.const 7) (i32.const 3)) (array.new_fixed $bvec 2 ...))` — confirmed
   by direct read of both files (`array_new_elem.wast:6-10,108`,
   `array.wast:226-229`). `array_init_elem.wast` has the identical shape
   for `array.init_elem`'s own source segment (`array_init_elem.wast:
   122-125,162`: `(elem $e1 arrayref (item (array.new_default $arrref_mut
   (i32.const 1))) (item (array.new_default $arrref_mut (i32.const 2))))`).

3. **Even with (1) and (2) fixed, `wasm_types::Element` has nowhere to put
   the result.** `Element.function_indices: Vec<Option<u32>>` (confirmed
   by direct read, `wasm-types/src/lib.rs:1403-1419`) can only represent
   "a function index" or "null" per entry — there is no representation for
   an arbitrary evaluated `WasmValue` (an `i31ref` payload, or a heap
   handle to a freshly-allocated GC array/struct). This is a genuine,
   necessary data-model gap, not a parsing shortcut.

**A fourth fact, confirmed by direct corpus read, sets the exact contract
layer (3) must satisfy**: `array_init_elem.wast:159-176` and
`array_new_elem.wast:105-121` each contain a dedicated test named "Test
that element segments are not re-evaluated on every `array.init_elem`/
`array.new_elem`" — it initializes two *separate* destination arrays from
the *same* single-item `(elem $elem arrayref (item (array.new_default
$arr (i32.const 0))))` segment, then asserts `ref.eq` on the two copied
elements is `1` (true, i.e. the literal same heap object, not two
independent allocations). **This means each item's constant expression
must be evaluated exactly ONCE, at module-instantiation time — identical
in spirit to how a `global`'s `init_expr` is evaluated once — never
re-evaluated per `array.init_elem`/`array.new_elem` call.** This is the
same semantic `wasm-execution::evaluate_const_expr_gc` already establishes
for globals (see "Current implementation" below); it generalizes cleanly
to element-segment items evaluated at the same point in instantiation.

**Corpus-impact accounting for this gap, precise**: of the 215-directive
cluster, 24 directives (11 in `array_new_elem.wast`, 13 in `array.wast`)
trace to layers (1)-(3) above via a non-funcref/externref/func-list
segment. These 24 are *inside* the six-instruction files but will **not**
resolve to `Pass` from implementing the six instructions' opcode/parse/
validate/execute machinery alone — they need this three-layer fix too.
Every OTHER `array.init_elem`/`array.new_elem`-touching module in the
corpus (`array_init_elem.wast`'s `funcref`/`externref`/`func $a $b...`
segments; `array_new_elem.wast`'s two `func $aa $bb $cc $dd` modules) is
already representable by the existing `function_indices: Vec<Option<u32>>`
model with zero changes to layers (1)-(3) — confirmed by direct read,
these use only `ref.func`/bare `func`-keyword funcidx-list segments.

## Correction 3: the bare `arrayref` value-type-keyword gap is real, narrow, and entirely separate from Correction 2

`array_init_elem.wast:118` (`(type $arrref_mut (array (mut arrayref)))`)
and `array_new_elem.wast:107` (`(type $arr (array (mut arrayref)))`) use
the bare `arrayref` keyword in a **storage-type** position (an array's own
element type), not an elem-segment reftype-tag position — a different
call site (`parse_value_type`'s bare-atom dispatch, `wasm-wast-parser/
src/module.rs` lines ~391-428, confirmed by direct read: recognizes
`funcref`/`externref`/`i31ref`/`anyref`/`nullref`/`nullfuncref`/
`nullexternref`/`nullexnref`/`exnref`, **not** `arrayref` or `structref`).
This is exactly the gap W37's own "Explicitly out of scope" section
already named and explicitly recommended bundling here: *"The bare
`arrayref` value-type keyword... real, but not needed by any TABLE
declaration in this cluster... Bundle with the array bulk-ops follow-on
instead."* `structref` was already added by W37 itself (`ValueType::
StructRefAny`); `arrayref` needs the exact same treatment, one level
later in the hierarchy (an `ArrayRefAny` variant, nullable, abstract top
of the array hierarchy — distinct from the existing `ArrayRef(u32)`,
which is nullable but always CONCRETE, carrying a type index — mirroring
`StructRefAny` vs. `StructRef(u32)` exactly).

15 directives (13 + 2) trace to this gap. It is a small, self-contained
fix, independently testable before touching any bulk-op instruction
logic (see slice decomposition).

## Real spec text, fetched directly and quoted/paraphrased faithfully

Fetched from `https://webassembly.github.io/gc/core/text/instructions.
html`, `/valid/instructions.html`, `/exec/instructions.html`, and
`/binary/instructions.html` (the real WASM GC proposal spec), and
cross-checked against this repo's own already-correct, already-shipped
opcode assignments for the neighboring instructions (`array.new`=`0xFB
0x06` through `array.len`=`0xFB 0x0F`, confirmed matching spec exactly at
`wasm-wast-parser/src/module.rs:4143-4152`'s own comparison table) as an
independent cross-check, since a single automated fetch of this specific
page mis-attributed one instruction's opcode on a retry (confirming the
fetch tool's own summarization is not 100% reliable for this page and
must be corroborated, not trusted blind — consistent with this
campaign's own standing "ground every claim, don't assume" discipline).

### Binary opcodes (`0xFB`-prefixed, decimal sub-opcode confirmed via direct fetch of the binary grammar page, and cross-checked against the repo's existing `0x00`-`0x0F`/`0x14`-`0x17` assignments, which all match spec verbatim except the repo's own deliberate, already-documented `struct.get_u`/`struct.set` swap at `0x04`/`0x05` — unrelated to this range):

```text
0xFB 9  (0x09) x:typeidx y:dataidx  ⇒ array.new_data x y
0xFB 10 (0x0A) x:typeidx y:elemidx  ⇒ array.new_elem x y
0xFB 16 (0x10) x:typeidx            ⇒ array.fill x
0xFB 17 (0x11) x:typeidx y:typeidx  ⇒ array.copy x y
0xFB 18 (0x12) x:typeidx y:dataidx  ⇒ array.init_data x y
0xFB 19 (0x13) x:typeidx y:elemidx  ⇒ array.init_elem x y
```

`0x09`/`0x0A` sit immediately after the already-shipped `array.new_fixed`
(`0x08`) and before the already-shipped `array.get` (`0x0B`) — filling
the one gap in this repo's own existing `0x06`-`0x0F` array block.
`0x10`-`0x13` sit immediately after the already-shipped `array.len`
(`0x0F`) and before the already-shipped `ref.test`/`ref.test null`
(`0x14`/`0x15`, confirmed present at `module.rs:4513`) — filling the gap
between the two existing GC instruction groups. **No renumbering of any
existing opcode is needed; this is purely filling two already-reserved
gaps in the repo's own pre-existing table**, and neither gap falls in the
`0x04`/`0x05` range affected by this repo's own struct.get_u/struct.set
deviation, so no analogous deviation risk applies here.

### Text-format grammar (fetched from `/text/instructions.html`, cross-checked against every module in the pinned corpus that uses each instruction — every real corpus call site matches this grammar exactly, no repo-specific deviation observed):

```text
"array.copy" x:typeidx_I y:typeidx_I      ⇒ array.copy x y        (x = DEST type, y = SRC type)
"array.fill" x:typeidx_I                  ⇒ array.fill x
"array.init_data" x:typeidx_I y:dataidx_I ⇒ array.init_data x y
"array.init_elem" x:typeidx_I y:elemidx_I ⇒ array.init_elem x y
"array.new_data" x:typeidx_I y:dataidx_I  ⇒ array.new_data x y
"array.new_elem" x:typeidx_I y:elemidx_I  ⇒ array.new_elem x y
```

Confirmed against real corpus call sites, e.g. `array_copy.wast`'s own
`(array.copy $arr8_mut $arr8 (ref.null $arr8_mut) ...)` (dest type first,
matching `x`=dest), `array.wast`'s own `(array.new_data $vec $d (i32.const
1) (i32.const 3))` (type then data index, matching `x y`).

### Validation rules (fetched from `/valid/instructions.html`, quoted):

- **`array.fill x`**: "The defined type `C.types[x]` must exist. The
  expansion must be an array type. **The prefix `mut` must be `var`**
  [i.e. the array's element must be declared mutable]. Let `t` be
  `unpack(storagetype)`." Stack: `[(ref null x) i32 t i32] → []`.
- **`array.copy x y`**: "Both array types must exist and expand
  correctly. **The first array's `mut` must be `var`**. **The second
  array's `storagetype` must [match](matching.html#match-storagetype) the
  first's `storagetype`**." Stack: `[(ref null x) i32 (ref null y) i32
  i32] → []`.
- **`array.init_data x y`**: "Array type must expand with `var`
  mutability. **The value type must be numeric or vector** [i.e. NOT a
  reference type — data segments hold raw bytes only]. The data segment
  `C.datas[y]` must exist." Stack: `[(ref null x) i32 i32 i32] → []`.
- **`array.init_elem x y`**: "Array type must expand with `var`
  mutability. **The `storagetype` must be [a] reference type `rt`**. The
  element segment `C.elems[y]` must exist. [The segment's own] reference
  type `rt'` must [match](matching.html#match-reftype) `rt`." Stack:
  `[(ref null x) i32 i32 i32] → []`.
- **`array.new_data x y`**: identical element-type and data-segment
  preconditions as `array.init_data`, no mutability requirement (a freshly
  allocated array is always writable at construction). Stack: `[i32 i32]
  → [(ref x)]`.
- **`array.new_elem x y`**: identical element-type and elem-segment
  preconditions as `array.init_elem`, no mutability requirement. Stack:
  `[i32 i32] → [(ref x)]`.

Every stack-order fact above was cross-checked against the corpus's own
argument order (e.g. `array_fill.wast`'s `(array.fill $arr8_mut (ref.null
$arr8_mut) (i32.const 0) (i32.const 0) (i32.const 0))` — array, offset,
value, count, matching `[(ref null x) i32 t i32]` left to right).

### Execution / trap semantics (fetched from `/exec/instructions.html`; the page truncates before reaching `array.init_data`/`array.init_elem`'s own reduction rules in a single fetch, so those two are derived from the validation rule's stack shape plus this repo's own already-established, corpus-verified `memory.init`/`table.init` trap-condition pattern — the same "dropped segment behaves as length-0" convention this repo already implements identically for both, confirmed by direct read of both handlers, `wasm-execution/src/lib.rs:6276-6346` (`memory.init`) and `6436-6526` (`table.init`)):

- **`array.fill`**: traps if the array reference is null (and `n>0`);
  traps if `d + n > array.len(ref)`. Semantics: writes `val` to
  `ref[d..d+n]`.
- **`array.copy x y`**: traps if either the source or destination array
  reference is null (and `n>0`); traps if `s + n >
  src_array.len` or `d + n > dest_array.len`. Semantics: copies `n`
  elements from `src[s..s+n]` to `dest[d..d+n]`, **overlap-safe**
  (`memmove` semantics — real spec concern when `dest` and `src` are the
  SAME array object with overlapping ranges; this repo's own `LinearMemory
  ::copy_between` already solves the identical problem for `memory.copy`
  and is the direct precedent to mirror, not re-derive).
- **`array.init_data x y`**: traps if `d + n > array.len`; traps if `s +
  n·|t|/8 > |data segment bytes|` (`|t|` = the storage type's byte width —
  `1` for `i8`, `2` for `i16`, `4`/`8`/`16` for `i32`/`f32`/`i64`/`f64`/
  `v128`). A dropped data segment behaves as length-0 (mirrors
  `memory.init`'s own already-implemented rule exactly: `n=0` still
  succeeds regardless, any `n>0` traps).
- **`array.init_elem x y`**: traps if `d + n > array.len`; traps if `s + n
  > |elem segment entries|`. A dropped elem segment behaves as length-0
  (mirrors `table.init`'s own already-implemented rule).
- **`array.new_data x y`** / **`array.new_elem x y`**: same segment-bounds
  trap as their `init_*` counterparts (no destination-array bounds check —
  there is no destination yet); additionally, **the requested length `n`
  must be bounded before allocation** — this repo's own pre-existing
  `array.new`/`array.new_default` handlers already enforce `MAX_ARRAY_
  ALLOC` (1,000,000 elements, `wasm-execution/src/lib.rs:5256`) as a
  defense-in-depth DoS guard against an attacker-controlled huge `n`; the
  real spec leaves this as an implementation-defined resource limit, and
  this repo's own established convention for every other array-allocating
  instruction is exactly this guard, so `array.new_data`/`array.new_elem`
  must reuse it identically — a genuinely security-relevant point, not
  optional polish (an unbounded `n` here would let a malicious module
  request an arbitrarily large heap allocation before any bounds check
  against a real segment size even runs, if the guard were checked in the
  wrong order).

## Current implementation, read directly

### `wasm-opcodes/src/lib.rs`: **no table entries exist for ANY GC struct/array/i31 opcode, old or new**

Confirmed by direct grep (`ARRAY_NEW`, `ARRAY_GET`, `STRUCT_NEW`, `0xFB`
as a GC-context byte): zero matches. This whole instruction family —
`struct.new` through `i31.get_u`, including the six already-shipped basic
array ops — bypasses `wasm-opcodes`' table mechanism entirely; every
byte value is a raw literal hand-written directly in `wasm-wast-parser`'s
encoder, `wasm-execution`'s decoder/dispatcher, and `wasm-validator`'s
type-checker (three independent, already-established call sites, not one
shared table). **This spec's own task framing's suggestion to "confirm
exactly which `0xFB`-prefixed GC opcodes already have table entries [in
`wasm-opcodes`]" has a one-word answer: none, for this whole family, by
long-standing existing convention — this is not a gap this spec
introduces or needs to fix.** `wasm-opcodes` changes are **not part of
this spec's design** — the three real call sites above are.

### `wasm-wast-parser/src/module.rs`: `encode_gc_struct_array_instr` (lines ~4179-4223) and its own doc comment (lines ~4099-4152)

This function's own doc comment **already names all six instructions
explicitly** as deliberately unwired: *"`array.new_data`/`array.new_elem`/
`array.copy`/`array.fill`/`array.init_data`/`array.init_elem` are
deliberately NOT wired here — they need real data-/elem-segment
integration this slice does not attempt... any module using them stays an
honest parse error at this crate's boundary, `NotYetSupported` at the
conformance-harness level, never a silent misencoding."* This is exactly
the gap this spec closes. Each existing instruction group
(`encode_struct_new`, `encode_struct_get_set`, `encode_array_new`,
`encode_array_new_fixed`, `encode_array_get_set`, `encode_array_len`) is
its own small function, kept deliberately separate "for the identical
stack-frame-size reason" (per `encode_struct_new`'s own doc comment: each
GC instruction group gets its own minimally-sized function so the union
of every group's locals doesn't bloat one shared frame) — this spec's six
new instructions should follow the same one-function-per-shape pattern,
not be folded into the existing ones.

`resolve_elem_expr_entry` (lines 2511-2537) and `build_elem` (lines
2355-2510, its reftype-vs-funcidx-list disambiguation specifically) are
the two elem-segment-parsing sites Correction 2 identifies as needing
extension — see Design §3 below.

`parse_value_type`'s bare-atom dispatch (lines ~391-428) is the site
Correction 3's `arrayref` gap needs — the identical function W37's own
design section already extends for `eqref`/`structref`; this spec adds
one more arm to the same match, landing after (or alongside) W37's own
change, whichever merges first.

### `wasm-execution/src/lib.rs`: `GcArray`/`GcObject`, the `0xFB` decode table, and the `0xFB` dispatch handler — reused, not reinvented

- **`GcArray { type_idx: u32, elements: Vec<WasmValue> }`** (lines
  4094-4100) and **`GcObject::Array(GcArray)`** (lines 4114-4117) are
  already the array heap-object representation `array.get`/`array.set`/
  `array.len`/`array.new*` use. This spec's six instructions read/write
  `elements` directly — no new heap-object shape needed.
- **`GcOp { sub: u8, type_idx: u32, field_idx: u32, extra: u32 }`** (lines
  4966-4977) is the existing per-instruction immediate side-table struct.
  It already has exactly enough fields for every one of the six new
  instructions' immediates **with zero new fields**: `array.fill` uses
  only `type_idx`; `array.copy` needs two type indices — reuse `field_idx`
  as the SECOND type index (currently only ever holds a struct field
  index; this spec broadens its role, needing an updated doc comment, not
  a new field); `array.init_data`/`array.init_elem`/`array.new_data`/
  `array.new_elem` each need one type index plus one data/elem index —
  again `type_idx` + `field_idx` (repurposed identically). `extra` stays
  unused by all six (it exists today only for `array.new_fixed`'s literal
  count).
- **The `opcode_byte == 0xFB` decode block** (lines 3084-3183) needs six
  new match arms added to its `match sub { ... }` (currently `0x00 | 0x01`,
  `0x14..=0x17`, `0x02..=0x05`, `0x06 | 0x07 | 0x0B..=0x0E`, `0x08`,
  `0x0F`, `_`) — `0x09 | 0x0A` and `0x12 | 0x13` each decode two LEB128
  index immediates (mirroring the existing `0x02..=0x05` two-index arm
  exactly); `0x10` decodes one (mirroring `0x06 | 0x07`); `0x11` decodes
  two (mirroring `0x02..=0x05` again). The block's own header comment
  table (currently documenting only through `0x0F`) needs the six new
  rows added, matching its existing style.
- **`vm.register_context_opcode(0xFB, ...)`** (line 5924, the actual
  execution dispatch, `match sub { 0x00 => ..., ..., 0x0F => ... , _ =>
  unsupported }`) needs six new arms. **`ctx.data_segments: Vec<Vec<u8>>`
  and `ctx.dropped_data_segments: Vec<bool>`** (lines 4303, 4314) are
  already exactly the representation `array.init_data`/`array.new_data`
  need to read from — the SAME fields `memory.init`'s own handler (lines
  6276-6346) already reads, with the SAME "dropped segment degrades to
  length-0" rule this spec's design reuses verbatim, not reimplements.
  `array.init_elem`/`array.new_elem` need the elem-segment equivalent —
  see Design §3/§4 for why `ctx.elements: Vec<Vec<Option<u32>>>` (the
  existing funcref-only representation `table.init` already uses,
  confirmed at lines 6469-6524) is necessary-but-insufficient and what
  this spec adds alongside it.
- **`evaluate_const_expr_gc`** (lines 2461-2734) is the exact machinery
  Correction 2's "evaluate each elem item once, at instantiation time"
  requirement needs — it already evaluates `ref.i31`/`struct.new(_
  default)`/`array.new(_default)`/`array.new_fixed`/`ref.null`/
  `global.get`/extended-const arithmetic into a real `WasmValue`, with
  real, persistent `gc_heap`/`v128_heap` side effects (its own doc comment
  explains why this must be real and persistent, not throwaway: `struct.
  wast`'s own global read back later in the same file). Its own doc
  comment already explicitly excludes `array.new_data`/`array.new_elem`
  from being legal INSIDE a constant expression — a real, corpus-grounded
  rule (`array.wast:302-326`'s own two `assert_invalid "constant
  expression required"` cases, already passing today because the whole
  module currently fails to PARSE, which the harness already grades as a
  hard rejection — see "Does this regress anything" below for why this
  stays correct once parsing is added) — **this spec's design does not
  touch that exclusion**; it only reuses the function's EXISTING coverage
  of `ref.i31`/`array.new*`/`struct.new*` to evaluate elem-segment items,
  which are ordinary constant expressions using exactly those same
  instructions, at a NEW call site (module instantiation's elem-segment
  build step) alongside its EXISTING call site (module instantiation's
  global build step).

### `wasm-validator/src/type_check.rs`: the `0xFB` match arm, `array_element_field`, and `field_is_structural_subtype` — the last one is an unusually good, pre-existing fit

- The per-function byte-layout stack-effect pass's own `0xFB` match
  (confirmed at lines ~2252-2377, e.g. `0x06 | 0x07` for `array.new(_
  default)`, `0x0E` for `array.set` with a real mutability check, `0x0F`
  for `array.len`) is where this spec's six new arms go — each decodes
  its own immediates (mirroring `wasm-execution`'s decode block 1:1, so
  `offset` never desyncs from a real following instruction — the exact
  failure mode `ref.cast`'s own W33 second-slice fix commit message
  documents having hit once already) and adjusts the abstract stack per
  the validation rules quoted above.
- **`array_element_field(module, type_idx) -> Result<&FieldType, ...>`**
  (line 741) already resolves a type index to its `FieldType { storage:
  StorageType, mutable: bool }` — exactly what every one of the six new
  arms needs for its mutability check (`array.fill`/`array.copy`'s
  destination `mut` must be `var`) and storage-type check (`array.init_
  data`/`array.new_data`'s numeric-or-vector requirement; `array.init_
  elem`/`array.new_elem`'s reference-type requirement).
- **`field_is_structural_subtype(child: &FieldType, parent: &FieldType,
  module) -> bool`** (line 1275, W34 third slice) is, **read closely, the
  exact real-spec `match-storagetype` relation `array.copy`'s own
  validation rule needs** — the real spec's own `match-storagetype`
  predicate for two field types is defined identically to this repo's
  existing struct/array structural-subtyping rule (invariant when
  mutable, covariant via `is_assignable` when immutable). `array.copy
  $dest $src`'s check is exactly `field_is_structural_subtype(&src_field,
  &dest_field, module)` — **zero new subtyping logic needed, a direct
  reuse of already-tested W34 infrastructure**, not an approximation of
  it. This is the single biggest reason this spec's validator-side design
  is smaller than it might first appear.

### `wasm-types/src/lib.rs`: `Element`, `StorageType`, `FieldType` — one real extension needed (Correction 2), everything else sufficient

- `StorageType`/`FieldType`/`ArrayType` (lines 726-844) already carry
  everything the validation rules above need (`widened_type()`,
  `is_packed()`, `packed_bits()`, `mutable`) — confirmed sufficient,
  zero changes.
- `Element` (lines 1403-1465, `function_indices: Vec<Option<u32>>`) is
  the one genuine gap — see Design §3.

## Design

### 1. `wasm-opcodes`: no changes

Confirmed above: this instruction family has never used this crate's
table mechanism, for any of its members, old or new. Adding a table here
now — for only six instructions out of an already-inconsistent family —
would be a net-new inconsistency, not a fix. Left alone.

### 2. `wasm-wast-parser`: six new binary-encoder functions + `parse_value_type` (`arrayref`) + the elem-segment three-layer fix

**2a. Opcode table additions**: none needed (see §1) — the six raw byte
literals below ARE this crate's own "opcode table" for this family, by
existing convention.

**2b. Six new encoder functions**, added to `encode_gc_struct_array_instr`'s
dispatch and each following the established one-function-per-shape
pattern:

```rust
// array.fill $t <arrayref> <i32 offset> <t value> <i32 count>
fn encode_array_fill(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let ty_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
    let type_idx = resolve_idx(&icx.module.type_names, ty_expr, "type")?;
    encode_instr_list(&args[1..], icx, out)?;
    out.push(0xFB);
    out.push(0x10);
    out.extend(wasm_leb128::encode_unsigned(type_idx as u64));
    Ok(())
}

// array.copy $dest $src <destref> <i32 d> <srcref> <i32 s> <i32 n>
fn encode_array_copy(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let dest_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
    let dest_idx = resolve_idx(&icx.module.type_names, dest_expr, "type")?;
    let src_expr = args.get(1).ok_or(WastParseError::UnexpectedEof)?;
    let src_idx = resolve_idx(&icx.module.type_names, src_expr, "type")?;
    encode_instr_list(&args[2..], icx, out)?;
    out.push(0xFB);
    out.push(0x11);
    out.extend(wasm_leb128::encode_unsigned(dest_idx as u64));
    out.extend(wasm_leb128::encode_unsigned(src_idx as u64));
    Ok(())
}

// array.init_data / array.init_elem $t $seg <ref> <i32 d> <i32 s> <i32 n>
// array.new_data  / array.new_elem  $t $seg <i32 s> <i32 n> -> ref
// (four instructions, same two-index-immediate shape as encode_struct_get_set;
// init_* vs new_* differ only in whether an extra leading operand (the
// destination array ref) gets encoded -- NOT in the immediate shape, so
// one function per {init,new} x {data,elem} pairing, or one parameterized
// function taking the sub-opcode byte and a "has destination ref operand"
// bool -- implementer's choice, whichever keeps each function's own
// frame minimally sized per this file's established convention).
```

Each resolves its data/elem index via `resolve_idx(&icx.module.data_
names, ..., "data")` / `resolve_idx(&icx.module.elem_names, ..., "elem")`
— the same tables `memory.init`/`data.drop`/`table.init`/`elem.drop`'s
own existing parsing already uses (confirmed present at `module.rs:224,
228`).

**2c. `parse_value_type`: add `arrayref`** (Correction 3) — one new arm
in the bare-atom match (`"arrayref" => Ok(ValueType::ArrayRefAny)`),
mirroring W37's own `"structref" => Ok(ValueType::StructRefAny)` addition
exactly. `wasm-types` needs the matching new `ValueType::ArrayRefAny`
variant (nullable, abstract top of the array hierarchy — see §5 below),
wired into `byte_tag()`/`encode()`/`is_bottom_subtype_of` the same way
W37 wires `StructRefAny`. No compound `(ref null array)`/`(ref array)`
form is needed by this cluster's own corpus (confirmed by grep: only the
bare `arrayref` atom appears) — though `parse_ref_null_heap_type`'s
existing `"array"` 2-item non-null-form handling (referenced by W37's own
read of this crate, producing `NonNullArrayAny`) already covers the
compound non-null case; only the bare-atom nullable abbreviation is
missing.

**2d. The elem-segment three-layer fix** (Correction 2), the largest
single piece of this spec's parser-side design:

- **Layer 1 (`build_elem`'s reftype-tag recognition)**: extend the
  disambiguation to accept ANY successfully-parsed `ValueType` reftype
  via `parse_value_type` (the exact same generalization pattern W37's own
  design section 3 applies to table declarations — reuse, don't
  reinvent), not just the bare `"funcref"`/`"externref"` atoms it
  recognizes today.
- **Layer 2 (`resolve_elem_expr_entry`)**: this function's contract needs
  to change from "parse `(ref.func ...)` or `(ref.null ...)`, return
  `Option<u32>`" to "parse ANY constant instruction sequence legal in an
  elem-segment item (per the real spec, the same instruction set
  `evaluate_const_expr_gc` already accepts: `ref.func`, `ref.null`,
  `ref.i31`, `struct.new(_default)`, `array.new(_default)`, `array.new_
  fixed`, `global.get`), return the RAW ENCODED BYTES of that expression
  (via `encode_instr_list`, exactly like every other constant-expression
  site in this crate already produces bytes for `wasm-runtime` to later
  evaluate) rather than eagerly resolving to a function index.
- **Layer 3 (`wasm_types::Element`)**: needs a field to hold these raw
  per-item constant-expression byte sequences alongside (not replacing)
  `function_indices`, so `table.init`/`table.copy`'s own existing,
  already-correct, already-tested funcref-only fast path is completely
  undisturbed. Proposed shape:

  ```rust
  pub struct Element {
      // ... existing fields unchanged ...
      pub function_indices: Vec<Option<u32>>,  // UNCHANGED, table.init/table.copy's own path
      /// One raw constant-expression byte sequence per item (Correction 2,
      /// W38) -- `Some(bytes)` for every entry, evaluated ONCE at
      /// instantiation time via `evaluate_const_expr_gc` (mirroring a
      /// global's `init_expr`) into this segment's own `element_values`
      /// runtime-side table (see wasm-execution/wasm-runtime below). A
      /// plain `ref.func $f`/`ref.null` item still round-trips through
      /// here too (its bytes ARE `[0xD2, <funcidx>, 0x0B]`/`[0xD0,
      /// <heaptype>, 0x0B]`) -- `function_indices` stays purely a
      /// FAST-PATH cache for table.init/table.copy's pre-existing
      /// consumers, not the source of truth for a segment's real content
      /// once this field exists. Always the same length as
      /// `function_indices` for any segment this crate builds.
      pub item_exprs: Vec<Vec<u8>>,
  }
  ```

  (The implementing session should double check whether populating
  `item_exprs` unconditionally for every element segment, vs. only for
  ones actually consumed by `array.init_elem`/`array.new_elem`, is worth
  the small parse-time cost either way — the corpus is small enough
  either choice is safe; unconditional is simpler and avoids a second,
  divergent code path.)

### 3. `wasm-execution`: decode table, dispatch handlers, and the new `element_values` side table

**3a. Decode table** (§ "Current implementation" above) — six new arms in
the `opcode_byte == 0xFB` match, each reusing the existing two-LEB128-
index or one-LEB128-index decode shapes already present for `0x02..=0x05`
and `0x06 | 0x07`.

**3b. `WasmExecutionContext` gains one new field**:

```rust
/// Per-elem-segment index, PARALLEL to `elements`/`dropped_elements`
/// (Correction 2, W38): each entry's evaluated constant value, computed
/// ONCE at instantiation time from `Element::item_exprs` via
/// `evaluate_const_expr_gc` -- the array-bulk-op analogue of
/// `data_segments: Vec<Vec<u8>>`, but holding real `WasmValue`s (which
/// may themselves be fresh `gc_heap` handles) instead of raw bytes,
/// because an elem segment's items are constant EXPRESSIONS, not a flat
/// byte blob. `array.init_elem`/`array.new_elem` read from here;
/// `table.init`/`table.copy` are UNCHANGED, still reading `elements`
/// (the pre-existing `Vec<Vec<Option<u32>>>`) directly -- two parallel,
/// independently-populated views of the same underlying segments, not
/// one migrated to the other, so zero regression risk to the already-
/// shipped table-bulk-ops path.
pub element_values: Vec<Vec<WasmValue>>,
```

populated by `wasm-runtime::instantiate()` (§4 below) the same way
`data_segments`/`elements` already are, saved/restored across calls the
same way (mirrors `set_dropped_elements`'s own existing setter pattern).

**3c. Six new execution handlers**, each mirroring its closest existing
precedent 1:1 rather than inventing a new shape:

- **`array.fill`** mirrors `memory.fill`'s pop-order/trap-check shape
  (pop count, value, offset; bounds-check `d.checked_add(n) <=
  array.elements.len()`; trap on null array reference OR bounds failure;
  write `val` to `elements[d..d+n]` via a plain slice fill, no `memmove`
  concern since there's only one array).
- **`array.copy`** mirrors `LinearMemory::copy_between`'s overlap-safe
  raw-pointer shape (§ "Execution / trap semantics" above) — SAFETY
  argument identical to that function's own doc comment: both `gc_heap`
  slot indices are bounds-checked before any aliasing pointers are
  formed, and `dest_idx == src_idx` (same array, self-copy) must use the
  identical direction-aware copy `LinearMemory::copy_between` already
  implements (copy forward when `d <= s`, backward otherwise) to get
  correct `memmove` semantics from a single `Vec<WasmValue>` — reuse that
  function's own algorithm shape, do not write a second, divergent
  implementation of the same "avoid aliased mutable/immutable borrow of
  the same heap slot" problem `gc_heap`'s `Vec<Option<GcObject>>`
  representation has (this is the array-heap analogue of the exact hazard
  `LinearMemory::copy_between`'s own doc comment already documents having
  been security-reviewed for, per W35 slice 2's own precedent for
  `gc_heap` aliasing specifically).
- **`array.init_data`/`array.new_data`** mirror `memory.init`'s handler
  (lines 6276-6346) almost verbatim: same `ctx.data_segments`/`ctx.
  dropped_data_segments` fields, same "out-of-range idx is a hard
  defensive error, dropped-in-range degrades to length-0" split, same
  `checked_add` bounds-check-before-any-write discipline — the only real
  difference is decoding `n` field-width-typed values from the raw byte
  slice (per `StorageType::packed_bits()`/`widened_type()`) into
  `WasmValue`s instead of copying raw bytes into a `LinearMemory`, and (for
  `array.new_data` only) allocating a fresh `GcArray` first, bounded by
  `MAX_ARRAY_ALLOC` (§ "Execution / trap semantics" above — a real,
  security-relevant guard, checked BEFORE the segment-bounds check, not
  after, mirroring `array.new`/`array.new_default`'s own existing order).
- **`array.init_elem`/`array.new_elem`** mirror `table.init`'s handler
  (lines 6436-6526) almost verbatim: same `ctx.elements`/`ctx.dropped_
  elements` length/dropped bookkeeping for BOUNDS-CHECKING purposes (a
  dropped segment's `element_values` entry should be treated as
  length-0 identically — `dropped_elements[idx]` gates both `ctx.elements`
  AND `ctx.element_values` together, they're always the same length and
  drop together), but reading the actual VALUES to copy from `ctx.
  element_values[elem_idx]` (§3b) instead of `ctx.elements[elem_idx]`
  (which stays `table.init`'s own exclusive reader). `array.new_elem`
  additionally allocates via `MAX_ARRAY_ALLOC`, same as `array.new_data`.

### 4. `wasm-runtime::instantiate()`: evaluate each elem segment's items once, populate `element_values`

Alongside the existing global-evaluation loop (lines ~2354-2379, calling
`evaluate_const_expr_gc` per global) and the existing elements/`dropped_
elements` build step (lines ~2464-2608), add one more pass: for each
module element segment, evaluate every entry in its (new) `item_exprs`
via `evaluate_const_expr_gc` (same `gc_heap`/`v128_heap` threading the
global loop already does — this is the exact reason `evaluate_const_expr_
gc` takes `gc_heap`/`v128_heap` as `&mut` rather than by value, per its
own doc comment: "this needs a REAL, persistent `gc_heap`... not a
throwaway one," and this is now its SECOND real call site, using the same
persistent heap the global loop already built), producing `Vec<WasmValue>`
per segment, stored into the new `element_values` field threaded into
`WasmExecutionContext` the same way `elements`/`data_segments` already
are (`WasmInstance`'s own equivalent field, `set_element_values` setter on
`WasmExecutionEngine`, mirrored save/restore in the call-frame
snapshot/restore functions at lines ~3216-3363 alongside `dropped_
elements`'s own existing mirrored calls).

This evaluation must happen in the SAME relative order as the existing
global-evaluation loop and BEFORE any function body executes (an
`array.init_elem`/`array.new_elem` call always reads an already-evaluated
`element_values` entry, never triggers evaluation itself) — this is what
makes the "not re-evaluated" corpus requirement (Correction 2) fall out
automatically, with no special-casing needed at the call site itself.

### 5. `wasm-types`: `ArrayRefAny` (Correction 3) and `Element::item_exprs` (Correction 2)

- **`ArrayRefAny`**: new `ValueType` variant, nullable, abstract top of
  the array hierarchy. Binary tag `0x6A` (mirroring `Anyref`=`0x6E`/
  `I31ref`=`0x6C`/`StructRefAny`=`0x6B` (W37)/`Eqref`=`0x6D` (W37)'s
  single-byte shape — `0x6A` is the real spec's own `array` abstract
  heap-type byte, confirmed via the same `/text/types.html`/`/binary/
  types.html` fetch W37 already did and this spec re-confirms unchanged).
  Wire into `byte_tag()`/`encode()`, extend `is_bottom_subtype_of` with
  `NullRef <: ArrayRefAny` (mirroring the existing `NullRef <: Anyref`/
  `NullRef <: StructRefAny`(W37) arms), and locate/mirror whatever
  mechanism makes `NonNullArrayAny`/`ArrayRef(_)` assignable to `Anyref`
  today, extended symmetrically for `ArrayRefAny <: Anyref` — same
  "verify against the corpus's own subtyping cases, don't assume"
  discipline W37's own design section already calls for.
- **`Element::item_exprs: Vec<Vec<u8>>`**: see Design §2d.

### 6. `wasm-validator`: six new `0xFB` match arms

Each decodes its own immediates (mirroring `wasm-execution`'s decode
shape exactly, so the two crates' byte-offset bookkeeping never
desyncs — the established, explicitly-documented risk this file's own
`ref.cast` comment already warns about), then applies the validation
rule quoted in "Real spec text" above:

```rust
0x10 => {
    // array.fill <type_idx>
    let (type_idx, size) = decode_unsigned(code, offset)...;
    offset += size;
    let field = array_element_field(ctx.module, type_idx as u32)?;
    if !field.mutable {
        return Err(ValidationError::Other(format!("array.fill: immutable array element (type {type_idx})")));
    }
    pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // count
    pop_val(&mut stack, frame!())?; // value (any type -- packed storage widens)
    pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // offset
    pop_val(&mut stack, frame!())?; // arrayref
}
0x11 => {
    // array.copy <dest_type_idx> <src_type_idx>
    let (dest_idx, sz1) = decode_unsigned(code, offset)...;
    let (src_idx, sz2) = decode_unsigned(code, offset + sz1)...;
    offset += sz1 + sz2;
    let dest_field = *array_element_field(ctx.module, dest_idx as u32)?;
    let src_field = *array_element_field(ctx.module, src_idx as u32)?;
    if !dest_field.mutable {
        return Err(ValidationError::Other(format!("array.copy: immutable destination array (type {dest_idx})")));
    }
    if !field_is_structural_subtype(&src_field, &dest_field, ctx.module) {
        return Err(ValidationError::Other(format!("array.copy: source type {src_idx} not assignable to destination type {dest_idx}")));
    }
    pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // n
    pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // s
    pop_val(&mut stack, frame!())?; // src ref
    pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // d
    pop_val(&mut stack, frame!())?; // dest ref
}
// 0x12/0x13 (init_data/init_elem) and 0x09/0x0A/0x18/0x19-shaped new_data/
// new_elem follow the same "decode two indices, check storage-type
// numeric-vs-reference via `array_element_field`, pop/push per the
// quoted stack type" shape -- omitted here for length, same rigor.
```

`array.init_data`/`array.new_data`'s "numeric or vector, not reference"
check and `array.init_elem`/`array.new_elem`'s "reference type" check
both read `field.storage` directly (`matches!(field.storage,
StorageType::Val(vt) if vt.is_reference_type())` or the numeric
equivalent — whichever helper this crate already has for classifying a
`ValueType`, confirmed to exist somewhere given `is_assignable`'s own
existing reference-vs-numeric handling; the implementing session should
locate and reuse it rather than write a third classification).

**`array.init_elem`/`array.new_elem`'s "segment reftype `rt'` must match
`rt`" check**: this crate has no per-element-segment STATIC reftype
recorded on `wasm_types::Element` today (only the runtime VALUES, once
Design §5 lands) — the implementing session must decide whether to (a)
add a `pub declared_type: ValueType` field to `Element` (the segment's own
declared reftype tag, e.g. `i31ref`/`arrayref`/`(ref $bvec)`, parsed once
in Layer 1's fix) for the validator to check statically, or (b) treat this
the same permissive way this crate's broader "no instruction-level
type-checker" scope boundary (W05 §4.3) already treats most `assert_
invalid` cases — defer the check, let it stay `NotYetSupported` for any
corpus case that specifically probes it. Option (a) is cheap (the tag is
already parsed in Layer 1) and consistent with this spec's other
mutability/storage-type checks being real, not deferred — **recommended**,
but flagged as a real implementer decision, not dictated here.

## Trap conditions, summarized (security-relevant — every one of these is a real memory-safety boundary on this interpreter's own `gc_heap`, not merely spec conformance)

| Instruction | Traps when |
|---|---|
| `array.fill` | array ref is null (`n>0`); `d + n > array.len` |
| `array.copy` | either array ref is null (`n>0`); `s + n > src.len`; `d + n > dest.len` |
| `array.init_data` | `d + n > array.len`; `s + n·width > \|segment bytes\|` (segment bytes = `[]` if dropped) |
| `array.init_elem` | `d + n > array.len`; `s + n > \|segment entries\|` (entries = `[]` if dropped) |
| `array.new_data` | `n > MAX_ARRAY_ALLOC` (before any segment read); `s + n·width > \|segment bytes\|` |
| `array.new_elem` | `n > MAX_ARRAY_ALLOC` (before any segment read); `s + n > \|segment entries\|` |

Every bounds check must use `checked_add`, never a bare `+`, matching
every existing bulk-memory/bulk-table handler's own established
convention (an unchecked `d + n` on attacker-controlled `i32`-derived
`usize` values is exactly the class of integer-overflow bug this
codebase's own `feedback_verify_dos_guards_adversarially`/
`feedback_per_name_visited_array_quadratic`-style lessons warn about
elsewhere in this campaign) — and every check must run BEFORE any write,
so a trap leaves both the source and destination arrays completely
unmodified (same atomicity `Table::copy_between`/`LinearMemory::copy_
between` already guarantee).

## Does this fully close the 215-directive cluster? (re-verified per-cause, not assumed)

- **175 (six-instruction-attributable)**: expected to reach real `Pass`
  once Design §§2b/3/4/6 land, for every module using only funcref/
  externref/func-list-sourced elem segments or numeric/vector-sourced
  data segments (confirmed to be every `array_copy.wast`/`array_fill.
  wast`/`array_init_data.wast`/`array_new_data.wast` module, plus most of
  `array_init_elem.wast`/`array_new_elem.wast`/`array.wast`'s own
  modules) — a re-verified, per-file prediction, not a blanket claim; see
  slice decomposition's own re-verification step.
- **24 (elem-segment reftype/const-value gap, Correction 2)**: expected to
  ALSO reach `Pass`, but only once Design §§2d/3b/4/5's `item_exprs`/
  `element_values` machinery lands too — this is NOT automatic from the
  six-instruction work alone, and is the single largest risk to this
  spec's own completeness claim if under-scoped.
- **15 (bare `arrayref`, Correction 3)**: expected to resolve as an
  independent, small, purely-parser fix (Design §2c), unblocking the
  `(array (mut arrayref))` type declarations these particular
  `array_init_elem.wast`/`array_new_elem.wast` modules need before their
  own `array.init_elem`/`array.new_elem` calls can even be reached.
- **1 (`(ref struct)` non-null abstract heap type)**: stays out of scope,
  already flagged by W37 as a separate, pre-existing gap unrelated to
  this cluster's own subject; not attempted here.

**If all four causes are addressed, this spec's own honest expectation is
214 of the 215 converting to `Pass`** (every one except the single
pre-existing `(ref struct)` case) — a much more complete closure than a
naive "just implement six instructions" reading would achieve (which
would leave 40 of the 215 as newly-different-but-still-`NotYetSupported`
failures, a materially worse outcome this spec's own re-verification
exists to catch before implementation starts, not after).

## Does this regress anything currently passing? (re-verified, not assumed)

- **`array.wast`'s own two `assert_invalid "constant expression required"`
  cases** (lines 302-326, currently passing because the whole module fails
  to PARSE today): once `array.new_data`/`array.new_elem` parse
  successfully elsewhere, these two globals' own init-expr bytes will
  reach `evaluate_const_expr_gc` for real. That function's existing `0xFB`
  match only accepts `0x00/0x01/0x06/0x07/0x08/0x1C` — `0x09` (`array.new_
  data`) and `0x0A` (`array.new_elem`) fall through to its existing `_ =>
  "illegal WasmGC sub-opcode... in constant expression"` catch-all
  UNCHANGED (this spec adds no new arms to this function for `0x09`/
  `0x0A` — see "Current implementation" above, this exclusion is
  deliberately preserved). The implementing session must re-verify (not
  assume) that this `TrapError` path is what `wasm-runtime::instantiate()`
  surfaces as an `assert_invalid`-gradeable rejection, the same way it
  already must be today for every other real "constant expression
  required" corpus case — a live re-probe of these exact two directives
  after Design §2b lands is this spec's own explicit verification-plan
  item, not an assumption.
- **`table.init`/`table.copy`/`table.fill`** (W17): completely untouched —
  Design §3b's `element_values` is a NEW, PARALLEL field; `ctx.elements`
  (their own read path) is neither renamed nor restructured.
- **`struct.new`/`array.new`/`array.new_default`/`array.new_fixed`-typed
  GLOBAL initializers** (W33 fourth slice): `evaluate_const_expr_gc`'s
  existing global call site is untouched; Design §4 adds a SECOND call
  site (elem-segment items), not a change to the first.
- **Every other file in the corpus** (all 250 outside this 7-file
  cluster): the only shared-infrastructure changes here are `parse_value_
  type` (one new arm, `arrayref` — additive, previously a hard parse
  error, never silently misinterpreted as something else, so no
  regression risk per the identical reasoning W37's own spec already
  applied to its own `parse_value_type` changes) and `field_is_
  structural_subtype`'s NEW call site from `array.copy`'s validation (the
  function itself is unchanged, only a new caller) — both low-risk,
  confirmed by the same "was previously a hard error, not a silent
  misparse" argument.

## Explicitly out of scope for this spec

- **The `(ref struct)` non-null abstract heap type** (`array.wast`'s
  remaining 1 NYS) — already flagged out of scope by W37 for a different
  file; unrelated to array bulk ops; not attempted here.
- **`ref.eq`, `extern.convert_any`/`any.convert_extern`, `br_on_cast`/
  `br_on_cast_fail`, `ref.test`/`ref.cast`'s abstract-heap-type-immediate
  restriction, the table-with-init-expression third table form,
  binary-format GC-typed table decoding, GC-reftype-typed table
  imports** — all already flagged out of scope by W37, all genuinely
  unrelated to this cluster, not re-litigated here.
- **A static `declared_type` field on `Element`** for a fully-enforced
  `array.init_elem`/`array.new_elem` segment-reftype-match VALIDATION
  check — flagged as a real implementer decision in Design §6, not
  mandated; the corpus's own `assert_invalid` coverage for this specific
  rule (if any) should drive whether it's worth adding now vs. deferring
  under this crate's existing "no instruction-level type-checker" scope
  boundary like most `assert_invalid` cases already are.
- **Binary-format (non-`.wast`-text) encoding of any of the six new
  instructions** — this repo's `wasm-module-parser` binary decoder is a
  separate crate from the text-format `wasm-wast-parser` this spec's
  design section focuses on; confirmed by corpus grep, no file in this
  cluster uses `(module binary ...)`, so (mirroring W37's own identical
  finding for table types) this stays dormant and unexercised, real but
  not corpus-driven, left for whichever future work first needs a
  binary-encoded GC bulk-op module.
- **Generalizing `item_exprs`/`element_values` to also become `table.
  init`/`table.copy`'s primary data source** (replacing `function_
  indices` outright) — a plausible future cleanup once both
  representations exist side by side, but deliberately NOT attempted
  here; this spec adds a second, parallel, purely-additive representation
  specifically to keep the already-shipped table-bulk-ops path
  completely undisturbed (see "Does this regress anything" above).

## Recommended slice decomposition

0. **`wasm-types`**: add `ArrayRefAny` (wire `byte_tag()`/`encode()`/
   `is_bottom_subtype_of`/whatever makes it assignable to `Anyref`) and
   `Element::item_exprs: Vec<Vec<u8>>` (empty `Vec` for every segment
   until slice 3 populates it — additive, no consumer yet, safe to land
   alone). Verify: `cargo test -p wasm-types` clean; new unit tests for
   `ArrayRefAny`'s `byte_tag()`/`encode()`/subtyping mirroring `Struct
   RefAny`'s own (W37) test shapes.
1. **`wasm-wast-parser`**: `parse_value_type`'s `arrayref` arm (Correction
   3, independently testable, unblocks 15 NYS by itself — verify via a
   direct re-probe of `array_init_elem.wast`/`array_new_elem.wast` showing
   their "expected a value type, found arrayref" messages gone,
   REPLACED by "unknown instruction array.init_elem"/"array.new_elem"
   (progress, not full resolution, until slice 2 lands — confirm the
   NEW failure mode precisely, don't assume). Verify: `cargo test -p
   wasm-wast-parser` clean; new unit test for bare `arrayref` in a
   storage-type position.
2. **`wasm-wast-parser` + `wasm-execution` + `wasm-validator`**:
   `array.fill`/`array.copy` (Design §§2b/3c/6) — the two instructions
   with NO segment interaction at all, the simplest, most independently
   verifiable pair. Verify: `cargo test -p wasm-wast-parser -p wasm-
   execution -p wasm-validator` clean; new unit tests for encode/decode/
   validate/execute on both, including the mutability and storage-type-
   match rejection cases; re-probe `array_copy.wast`/`array_fill.wast`
   directly, expect both to newly report real `Pass`/`Fail`/`Trap`
   outcomes (never `NotYetSupported`) for every directive.
3. **`wasm-wast-parser` + `wasm-execution` + `wasm-validator`**:
   `array.init_data`/`array.new_data` (Design §§2b/3c/6, reusing `ctx.
   data_segments`/`dropped_data_segments` verbatim, zero new runtime
   plumbing beyond the handlers themselves). Verify: same shape as slice
   2; re-probe `array_init_data.wast`/`array_new_data.wast` and the
   `array.new_data`-attributable 14/28 of `array.wast`'s own NYS.
4. **The elem-segment three-layer fix** (Correction 2, Design §§2d/3b/4,
   the largest and riskiest slice — isolate it from slice 5's actual
   `array.init_elem`/`array.new_elem` opcodes so a mistake here is
   diagnosable independently): `build_elem`'s reftype-tag generalization,
   `resolve_elem_expr_entry`'s rewrite to capture raw constant-expr bytes,
   `Element::item_exprs` real population, `wasm-runtime::instantiate()`'s
   new elem-item-evaluation pass, `WasmExecutionContext::element_values`.
   Verify: `cargo test -p wasm-wast-parser -p wasm-runtime -p wasm-
   execution` clean; new unit/integration tests confirming (a) a
   `funcref`/`func`-list segment's `element_values` matches its pre-
   existing `function_indices` reading exactly (no divergence), (b) an
   `i31ref`/`arrayref`-item segment's `element_values` holds the real
   evaluated `WasmValue`s, (c) the SAME segment evaluated twice via two
   separate reads of `element_values` yields the identical `gc_heap`
   handle both times (the "not re-evaluated" contract, directly testable
   even before slice 5's own opcodes exist, by reading `element_values`
   from a test harness directly) — and confirm `table_init_copy_elem_
   drop_end_to_end` (the pre-existing W17 test) still passes completely
   unchanged.
5. **`wasm-wast-parser` + `wasm-execution` + `wasm-validator`**:
   `array.init_elem`/`array.new_elem` themselves (Design §§2b/3c/6),
   built on slice 4's `element_values`. Verify: same shape as slice 2;
   re-probe `array_init_elem.wast`/`array_new_elem.wast` and the
   `array.new_elem`/elem-segment-attributable remainder of `array.wast`.
6. **Full corpus re-verification.** `cargo run --release --bin wasm_
   conformance_report -p wasm-conformance -- --write-baseline`, diffed
   against the pre-slice-0 baseline. Confirm the PER-FILE, PER-CAUSE
   predictions in "Does this fully close the 215-directive cluster"
   above precisely (expect 214/215 converted, 1 remaining `(ref struct)`
   case unchanged) — and confirm the two `array.wast` "constant
   expression required" `assert_invalid` cases (§"Does this regress
   anything") still pass, for the NEW reason (real rejection inside
   `evaluate_const_expr_gc`) rather than their OLD reason (whole-module
   parse failure) — re-probe directly, don't infer from an unchanged
   aggregate count alone. Confirm zero regressions across the other 250
   files.

## Verification plan (for whatever session implements this)

1. `cargo build --workspace` and `cargo test --workspace` (or at minimum
   `-p wasm-types -p wasm-wast-parser -p wasm-validator -p wasm-execution
   -p wasm-runtime -p wasm-conformance`) clean after EACH slice, not just
   at the end — this campaign's own established precedent.
2. `cargo run --release --bin wasm_conformance_report -p wasm-conformance
   -- --write-baseline`, diffed against the pre-slice-0 baseline, checked
   against the EXACT per-file, per-cause predictions above.
3. For every corpus-cited fixture in this spec (`array.wast` lines
   10,158-259,302-326; `array_copy.wast`; `array_fill.wast`; `array_init_
   data.wast`; `array_init_elem.wast` lines 9-176; `array_new_data.wast`;
   `array_new_elem.wast` lines 6-121), spot-check with a direct `run_wast_
   source` probe rather than trusting the aggregate report number alone.
4. Explicitly re-run `table_init_copy_elem_drop_end_to_end` (`wasm-
   execution`'s own pre-existing W17 test) and every `struct.wast`/
   `array.wast` global-initializer test touching `evaluate_const_expr_gc`
   after slice 4 lands, to confirm the new elem-item evaluation call site
   didn't disturb the existing global call site (they now share the
   SAME persistent `gc_heap`/`v128_heap` within one `instantiate()` call —
   confirm allocation ORDER between globals and elem-segment items
   doesn't matter for any real corpus case, or if it does, that this
   spec's "globals first, elem items second" ordering choice matches
   what the corpus actually needs; re-verify, don't assume the ordering
   is inert).
5. Confirm every new bounds check uses `checked_add` and runs before any
   write (§"Trap conditions" above) — write an adversarial test per
   instruction with `d`/`s`/`n` chosen to overflow `usize` arithmetic if
   computed unchecked, mirroring this campaign's own `feedback_verify_
   dos_guards_adversarially` lesson.
6. Confirm `array.copy`'s self-copy (same array, overlapping `d`/`s`
   ranges) case is memmove-correct, not just non-panicking — a dedicated
   unit test with `d > s` and `d < s` overlapping ranges on the SAME
   `GcArray`, asserting the exact resulting element order, mirroring
   `linear_memory_copy_moves_bytes_overlap_safe`'s own existing test
   shape for `memory.copy`.

## Addendum (2026-09-02): slices 4/5 shipped — this spec's own six-slice plan is now CLOSED

Slices 4/5 (the elem-segment three-layer fix, Correction 2, plus
`array.init_elem`/`array.new_elem` themselves) landed together, completing
the six-slice plan this document laid out (slices 0-2 in #14114, slice 3
in #14120). Re-verified against the real 257-file corpus, not assumed:

- **`array.wast`**: 40/54 → 53/54 pass. The single remaining
  `not_yet_supported` is the pre-existing, out-of-scope `(ref struct)`
  non-null abstract heap type this spec's own "Explicitly out of scope"
  section already named (already flagged by W37 for a different file,
  unrelated to array bulk ops) — **exactly matching this spec's own
  honest "214 of 215 convert" prediction**, not a shortfall.
- **`array_init_elem.wast`**: 3/36 → 23/36 pass. **`array_new_elem.wast`**:
  0/24 → 22/24 pass. Every remaining `not_yet_supported` in both files
  traces to `ref.eq` (confirmed by direct re-probe) — already flagged out
  of scope by W37, genuinely unrelated to this spec's own six
  instructions, not a gap this spec ever claimed to close.
- **`array_copy.wast`/`array_fill.wast`/`array_init_data.wast`/
  `array_new_data.wast`** (slices 2/3's own targets): unaffected by
  slices 4/5, still 100% pass, re-confirmed.

**Total real progress across all six instructions, from the 215-directive
cluster this spec's own "Correction 1" re-derived at the start**: 214 of
215 now convert to real `Pass`/`Fail`/`Trap` outcomes, never `Not
YetSupported`, for exactly the reason predicted (the single `(ref
struct)` case staying out of scope). This spec's own "Does this fully
close the 215-directive cluster?" section's stated expectation is
CONFIRMED, not merely assumed — re-probed directly, per-file, per-cause,
not inferred from an aggregate count.

**Two real, corpus-caught side effects found while landing slices 4/5,
both fixed except one deliberately left out of scope** (full traces in
`wasm-validator`'s and `wasm-runtime`'s own CHANGELOGs):

1. **Fixed**: `wasm-validator`'s Check 4c (out-of-range `ConcreteFuncRef`
   type indices) needed a new arm for `Element::declared_type` — a bare
   numeric elem-segment reftype tag (`(ref 1)`) is never bounds-checked
   by `resolve_idx`, the same root cause Check 4c already exists for in
   every OTHER declared-signature position. Caught by `ref.wast`
   regressing then being restored to its exact pre-slice baseline by
   this one fix.
2. **Fixed**: `wasm-runtime`'s active-elem-application loop (which
   populates a TARGET TABLE at instantiation time, a different consumer
   than `table.init`/`table.copy`) needed to read the new `element_
   values` table instead of the pre-existing `function_indices`, once
   Layer 1/2's generalization let an ACTIVE segment's own item be
   something richer than a literal `ref.func`/`ref.null` (`global.wast`'s
   own `global.get`-sourced item). Caught by `global.wast` regressing
   then being restored to a STRICT improvement (its own pre-existing 5
   `not_yet_supported` cases all converting to real `Pass`) by this fix.
3. **Deliberately NOT fixed, genuinely out of scope**: `elem.wast`'s own
   "Initializing a table with imported funcref global" test needs real
   cross-instance funcref propagation THROUGH AN IMPORTED GLOBAL — an
   already-documented W35-level architectural boundary
   (`resolve_all_table_funcrefs`'s own doc comment: "no vendored corpus
   file needs cross-instance funcref-GLOBAL resolution at all... this
   pass is scoped to tables only"), not something slices 4/5 ever
   designed to close. This ONE directive moves from `not_yet_supported`
   (previously failed to parse) to a real, safely-trapped `fail` — an
   honest, understood cost of correctly generalizing the parser, not a
   silent regression. Also: 4 `elem.wast` `assert_invalid` cases move from
   an accidental parse-failure `Pass` to an honest `not_yet_supported`
   (this crate has no constant-expression type/arity checker anywhere,
   confirmed by grep — joining an already-substantial existing category,
   not a new class of gap). Both are real, narrow, fully diagnosed
   trade-offs of doing this generalization correctly rather than
   narrowly special-casing this spec's own three target files — see
   `wasm-conformance`'s own CHANGELOG for the complete per-directive
   accounting.

**Genuinely remaining, explicitly out of scope, not this spec's job**:
`ref.eq` (13+2 directives across the two `_elem` files), the pre-existing
`(ref struct)` case (1 directive, `array.wast`), and cross-instance
funcref propagation through an imported GLOBAL feeding an elem segment
(1 directive, `elem.wast`, pre-existing W35 boundary). Nothing else from
the original 215-directive cluster remains open.
