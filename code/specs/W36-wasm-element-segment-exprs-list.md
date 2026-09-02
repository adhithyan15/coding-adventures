# W36 — WASM element-segment "exprs-list" form

## Purpose and how this slice was chosen

`code/specs/W07-wasm-post-mvp-epics.md`'s "Addendum 2 (2026-09-02)" — the
first per-file-grounded survey of the ~4048 `not_yet_supported` (NYS)
directives left after the W35 epic closed — names the element segment
"exprs-list" form (a real, specified bulk-memory-operations-proposal
encoding, later folded into core WASM 2.0/3.0) as "the single most
important finding" and "the single highest-leverage item in the entire
remaining backlog": it claims `table_copy.wast`, `table_copy64.wast`,
`table_init.wast`, `table_init64.wast` alone lose 2130 of the 4048 total
NYS directives (53%) to this one gap, cascading from a handful of
top-of-file SETUP modules failing to parse/decode.

This document does what this campaign's own convention (W32-W35) requires
before a spec is trusted: it re-verifies that claim directly against the
pinned corpus and the actual current source, not re-assumed from the
addendum's prose — using the SAME direct-probe method the addendum itself
used (`wasm_conformance::run_wast_source` on each file, reading every
distinct `NotYetSupported` message and its exact source position). That
re-verification finds the addendum's diagnosis of *this specific number*
is wrong, and the real fix for the 2130-directive item is a different,
much smaller, unrelated bug. The exprs-list gap is still real and still
worth fixing — it is confirmed as the cause of a meaningful slice of
`elem.wast`'s own remaining failures, and a couple of adjacent files — so
this document specs it properly. But the headline "2000+ directives, do
this first" framing needs correcting before anyone sequences work against
it.

### Correction to Addendum 2: `table_copy*.wast`/`table_init*.wast`/`bulk.wast`'s 2130+85 NYS directives are NOT primarily an exprs-list problem

Direct probe of the current `main`-tip source (verified live against the
pinned corpus, method: build a throwaway `wasm-conformance` bin calling
`run_wast_source` on each file and slicing the exact source substring at
each error's own reported byte position — not guessed from the message
text alone) shows:

- **`table_copy.wast`** (566 NYS) and **`table_copy64.wast`** (566 NYS):
  every one of the 11 distinct failing top-level `module` definitions in
  each file fails at the exact same construct —
  ```wat
  (func (export "run") (param $targetOffs i32) (param $srcOffs i32) (param $len i32)
    (table.copy (local.get $targetOffs) (local.get $srcOffs) (local.get $len))))
  ```
  `table.copy` here has **zero** leading table-index atoms (both the
  destination and source table default to table 0 — the real spec's own
  backward-compatible abbreviation, confirmed against the real spec text,
  §"The real grammar" below). The error, confirmed by printing the exact
  source slice at the reported position, points at the FIRST stack
  operand, `(local.get $targetOffs)` — not at anything to do with element
  segments at all.
- **`table_init.wast`** (499 NYS) and **`table_init64.wast`** (499 NYS):
  the dominant failing construct (confirmed the same way) is
  ```wat
  (table.init 1 (i32.const 12) (i32.const 1) (i32.const 1))
  ```
  — `table.init`'s own real backward-compatible abbreviation: ONE leading
  atom names the elem segment, with the table index defaulting to 0 (see
  below). `table_init.wast` additionally has 2 NYS directives from a
  genuinely different, GC-specific cause (an `arrayref`-typed exprs-list
  entry using `array.new_default` as its constant expression — see
  "Explicitly out of scope," this spec's own scoped `read_elem_expr_entry`
  correctly does not attempt this) and `table_init64.wast` shares the same
  2.
- **`bulk.wast`** (42 NYS): the SAME two shapes —
  `(table.copy (local.get 0) (local.get 1) (local.get 2))` (zero explicit
  table indices) and `(table.init 0 (local.get 0) (local.get 1)
  (local.get 2))` / `(table.init $p (i32.const 0) (i32.const 0)
  (local.get $len))` (elem-only, table implicit). The `$p`-named case is
  the same bug wearing a different mask: `$p` is a real, valid elem-segment
  name, but the current parser (see below) always resolves the FIRST
  atom against `table_names`, producing `unknown table identifier "$p"`
  instead of correctly treating it as the (only) elem-segment index.

**Root cause, confirmed by direct read**: `code/packages/rust/
wasm-wast-parser/src/module.rs`'s `encode_table_init_flat` (line 3809) and
`encode_table_copy_flat` (line 3826) — both reached from the FOLDED
instruction dispatcher (`encode_flat_instr`, lines 4198-4202: `if name ==
"table.init" { return encode_table_init_flat(...) } if name ==
"table.copy" { return encode_table_copy_flat(...) }`):

```rust
fn encode_table_init_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let table_idx = resolve_idx(&icx.module.table_names, expect_get(args, 0)?, "table")?;
    let elem_idx = resolve_idx(&icx.module.elem_names, expect_get(args, 1)?, "elem")?;
    encode_instr_list(&args[2..], icx, out)?;
    ...
}

fn encode_table_copy_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let dst_table_idx = resolve_idx(&icx.module.table_names, expect_get(args, 0)?, "table")?;
    let src_table_idx = resolve_idx(&icx.module.table_names, expect_get(args, 1)?, "table")?;
    encode_instr_list(&args[2..], icx, out)?;
    ...
}
```

Both functions unconditionally treat `args[0]`/`args[1]` as index atoms —
there is no arity detection at all for the real grammar's optional
leading indices. When `table.copy` is written with zero leading atoms,
`args[0]` is actually the first STACK OPERAND, `(local.get
$targetOffs)` — an `SExpr::List`, not an atom — and `resolve_idx`'s
generic non-atom branch (`module.rs:257-261`) produces exactly the
observed `"expected an index or $identifier, found list"`. When
`table.init` is written with ONE leading atom (elem-only shorthand),
`args[0]` (the elem index, e.g. `1` or `$p`) is wrongly resolved against
`table_names` instead of `elem_names`, then `args[1]` (the first real
stack operand) is wrongly fed to `resolve_idx` as if it were the elem
index — producing either `unknown table identifier` (if the atom happens
not to exist in `table_names`, as with `$p`) or the same `"found list"`
error (once past the first `resolve_idx` call) depending on which atom
existed in which namespace.

The doc comment immediately above the OTHER (non-folded, "flat/stream")
`table.init`/`table.copy` handling at `module.rs:2751-2761` explicitly
claims, citing `code/specs/W17-wasm-bulk-table-ops.md`'s own census:
*"`table_init.wast`/`table_copy.wast` only ever use folded syntax"* — true
as far as it goes, but that census apparently never separately checked
whether the folded syntax always supplies BOTH optional index atoms. It
does not, for exactly the eleven-plus-N modules identified above.

**The real grammar** (WebAssembly core spec, text format, table
instructions — confirmed by direct fetch of the spec text, not
re-guessed):

```text
'table.init' x:tableidx_I y:elemidx_I  ⟹  table.init x y
'table.init' y:elemidx_I               ≡  table.init 0 y      (table index optional, defaults to 0)

'table.copy' x₁:tableidx_I x₂:tableidx_I  ⟹  table.copy x₁ x₂
'table.copy'                              ≡  table.copy 0 0   (both indices optional, default to 0)
```

**This is a self-contained, `wasm-wast-parser`-only fix, unrelated to
element-segment content, `wasm_types::Element`, or anything in
`wasm-module-parser`/`wasm-runtime`/`wasm-execution`.** The fix is purely
arity detection before the existing `resolve_idx` calls:

- `encode_table_init_flat`: if `args[0]` is not an atom (or, more
  precisely, if there are fewer than 2 leading atoms before the first
  list), treat the single leading atom as the elem index with table index
  implicitly `0`; otherwise (2 leading atoms) keep today's behavior. A
  `table.init` folded form always needs an elem index (it can never be
  fully index-free, unlike `table.copy`), so the only ambiguity is
  "1 atom or 2."
- `encode_table_copy_flat`: if `args[0]` is not an atom at all, both
  indices default to `0` and zero atoms are consumed; otherwise (2 leading
  atoms) keep today's behavior. The real grammar has no "exactly one
  index" abbreviation for `table.copy`, so the check is binary (0 or 2
  leading atoms), never ambiguous.

This is sized **S** — a few-line fix in two existing functions, no new
types, no ripple into any other crate — and, being unrelated to this
spec's own exprs-list subject, is **out of this spec's own slice
decomposition** (§"Recommended slice decomposition" still lists it, as
slice 0, precisely because of its outsized leverage-to-effort ratio: it
alone likely accounts for far more of the 4048-directive backlog than the
exprs-list fix below, once `table_copy*.wast`/`table_init*.wast`/
`bulk.wast` are re-probed after it lands). Whoever implements this spec
should do slice 0 FIRST and independently verify its own corpus delta
before touching anything exprs-list-related, since the two fixes are
otherwise unrelated and slice 0's delta would otherwise contaminate the
exprs-list slices' own "did this change anything it shouldn't have"
verification.

## What the exprs-list gap actually is (grounded in the real spec text)

Despite the correction above, the exprs-list gap itself is real, still
needed, and reasonably scoped. Two real, specified encodings exist for an
element segment's *contents* in every language the WASM ecosystem uses to
describe it:

1. **funcidx-list** (the original MVP + reference-types-proposal shape,
   already fully supported by this interpreter): a plain list of function
   indices. Every entry names a function directly; a segment using this
   form can only ever hold non-null funcref values.
2. **exprs-list** (bulk-memory-operations proposal, later folded into
   WASM 2.0/3.0's core spec): a list of full constant expressions, each
   producing a reference value. This is what lets an element segment be
   typed with an arbitrary reference type (funcref, externref, and, once
   this repo's other GC/table-reftype gaps close, any GC reftype) and
   hold explicit null entries (`ref.null`), not just non-null function
   references.

### Binary format — the 3-bit flags byte, all 8 combinations

Confirmed by direct fetch of `https://webassembly.github.io/spec/core/
binary/modules.html#element-section` (quoted verbatim, not paraphrased):

```text
elem ::=
  | 0x00 e_o:expr y*:list(funcidx)
      => elem (ref func) (ref.func y)* (active 0 e_o)
  | 0x01 rt:elemkind y*:list(funcidx)
      => elem rt (ref.func y)* passive
  | 0x02 x:tableidx e:expr rt:elemkind y*:list(funcidx)
      => elem rt (ref.func y)* (active x e)
  | 0x03 rt:elemkind y*:list(funcidx)
      => elem rt (ref.func y)* declare
  | 0x04 e_o:expr e*:list(expr)
      => elem (ref null func) e* (active 0 e_o)
  | 0x05 rt:reftype e*:list(expr)
      => elem rt e* passive
  | 0x06 x:tableidx e_o:expr rt:reftype e*:list(expr)
      => elem rt e* (active x e_o)
  | 0x07 rt:reftype e*:list(expr)
      => elem rt e* declare
```

Bit semantics (the leading integer is a 3-bit bitfield): bit 0 separates
active (`0`) from passive/declarative (`1`); bit 1, for a passive/
declarative segment, separates passive (`0`) from declarative (`1`) — for
an active segment it instead signals "an explicit table index follows"
(`1`) vs. "implicit table 0" (`0`); bit 2 separates funcidx-list-with-
`elemkind` (`0`) from exprs-list-with-full-`reftype` (`1`). `elemkind` is
always the single byte `0x00` (meaning `funcref`) in every encoder that
exists; `reftype` is the richer encoding below.

**Currently implemented** (`code/packages/rust/wasm-module-parser/src/
lib.rs`, `parse_element_section`, lines 1150-1216, confirmed by direct
read): flags `0`, `1`, `2` (all three funcidx-list forms — active-
implicit-table, passive, active-explicit-table) and `5` (passive,
exprs-list, hardcoded to accept only the single-byte `funcref` reftype,
`0x70`). Flags `3`, `4`, `6`, `7` hit the catch-all `other => Err(...
"unsupported element segment mode flags {other} (only 0/1/2/5
supported)")` at line 1196. `read_elem_expr_entry` (lines 1228-1247)
already decodes exprs-list ENTRIES generically — `0xD2 <funcidx>` (`ref.
func`, → `Some(idx)`) or `0xD0 <heaptype>` (`ref.null`, → `None`) — and is
already reused as-is by this spec's design; it needs no changes.

**Reftype encoding** (needed to read flags 4/6/7's `rt:reftype`, and to
generalize flag 5's currently-hardcoded single-byte check): confirmed
against the same spec page's `reftype`/`heaptype` productions —
`0x70` ⇒ `funcref`, `0x6F` ⇒ `externref`, `0x64 ht` ⇒ `(ref ht)`
(non-null), `0x63 ht` ⇒ `(ref null ht)`. The pinned corpus (see below)
only ever uses `0x70`/`0x6F` directly, or `0x64 0x70` (`(ref func)`,
confirmed in `elem.wast`'s own already-decoded binary fixture, line 585:
`"\07\64\70\01\d2\00\0b"` — flag `0x07`/declare-exprs, reftype `0x64 0x70`
= non-null `(ref func)`, count `1`, entry `ref.func 0`). No corpus file
uses a concrete (non-abstract) heap type or `externref`'s `(ref extern)`/
`(ref null extern)` compound form in an elem segment's reftype position —
scope the binary reftype reader to `0x70`/`0x6F` bare, plus `0x63`/`0x64`
followed by the single abstract heap type byte `0x70` (`func`) or `0x6F`
(`extern`), rejecting anything else with a clear, loud error (matching
this parser's own established "scoped to what the corpus needs, reject
everything else loudly" discipline — see `read_elem_expr_entry`'s own doc
comment for precedent).

### Text format — the folded/unfolded grammar

Confirmed by direct fetch of `https://webassembly.github.io/spec/core/
text/modules.html#element-segments` (quoted verbatim):

```text
elem_I ::= "(elem" id? elemlist_I ")"                       ⇒ passive
         | "(elem" id? tableuse_I offsetexpr_I elemlist_I ")" ⇒ active(table, offset)
         | "(elem" id? "declare" elemlist_I ")"              ⇒ declarative

elemlist_I ::= rt:reftype_I e*:list(elemexpr_I)              ⇒ (rt, e*)
             | "func" x*:list(funcidx_I)                     ≡ (ref func) ((ref.func x)*)   [funcidx-list shorthand]

elemexpr_I ::= "(item" e:expr_I ")"                          ⇒ e
             | foldedinstr_I                                 ≡ (item foldedinstr_I)          [bare-instruction shorthand]

offsetexpr_I ::= "(offset" e:expr_I ")"                      ⇒ e
                | foldedinstr_I                              ≡ (offset foldedinstr_I)         [bare-instruction shorthand]
```

Plus one legacy-compatibility abbreviation: when the table use is omitted
(implicit table 0), the `func` keyword itself may ALSO be omitted from
the funcidx-list form — the oldest MVP shape, `(elem (i32.const 0) $f0
$f1 ...)` — already fully supported by this repo (`build_elem`, see
below).

### Currently implemented (`wasm-wast-parser`, confirmed by direct read of `code/packages/rust/wasm-wast-parser/src/module.rs`)

`build_elem` (lines 2231-2354) already, correctly, supports:

- The funcidx-list form, both with and without the `func` keyword, active
  (with or without an explicit `(table ...)` clause) or passive
  (`is_passive`, lines 2274-2284).
- **Declarative segments** (`is_declarative`, lines 2261-2264) — a real,
  deliberate design choice already in place: this repo's `wasm_types::
  Element` has no distinct third variant for "declarative" (only
  `is_passive: bool`); a declarative segment is represented as `is_
  passive: true`. The doc comment at lines 2239-2260 explains why this is
  sound for every case the corpus exercises (nothing ever `table.init`s
  or `elem.drop`s a declarative segment; `wasm-validator` does not
  separately enforce the real spec's "declared functions only" `ref.func`
  validity rule). **This spec adopts the identical convention for the
  BINARY decoder's flags 3/7** — no representation change needed.
- **Passive exprs-list** (`use_exprs`, lines 2305-2322, restricted to a
  leading bare `funcref`/`externref` keyword) — confirmed working by
  direct probe against `table_copy.wast`'s own `(elem funcref (ref.func
  2) (ref.func 7) ...)` fixtures (these parse and build correctly today;
  they are NOT part of that file's NYS count).
- `resolve_elem_expr_entry` (lines 2362-2388): decodes a single exprs-list
  entry — `(ref.func $x)` → `Some(idx)`, `(ref.null T)` → `None` — mirrors
  `read_elem_expr_entry`'s binary-side scope exactly.

**What's actually missing**, confirmed by direct probe against the pinned
corpus (method: same throwaway-probe technique as the correction above):

1. **An ACTIVE segment using the exprs-list form is unconditionally
   rejected** — `module.rs:2337-2343`:
   ```rust
   if !is_passive && use_exprs {
       return Err(WastParseError::UnexpectedToken {
           ...
           found: "an active element segment using the exprs-list (funcref/externref) form".to_string(),
           expected: "an active segment to use a plain function-index list instead (exprs-list is only supported for passive segments)",
       });
   }
   ```
   with a `/security-review` comment (lines 2323-2336) explicitly citing
   `W17-wasm-bulk-table-ops.md`'s own (now-superseded) census that no
   vendored corpus file needs this. That census is now stale: direct probe
   of the CURRENT pinned corpus finds real, live uses in `elem.wast` (its
   own dominant NYS cause, both implicit- and explicit-table forms:
   `(elem (i32.const 0) funcref (ref.null func))`,
   `(elem (table 0) (i32.const 0) funcref (ref.func 0))`,
   `(elem (i32.const 0) externref (ref.null extern))`), and a handful of
   hits each in `global.wast` (6) and `ref_func.wast` (1).
2. **The `(item ...)` explicit exprs-list-entry wrapper is not
   recognized at all** — `resolve_elem_expr_entry` (lines 2362-2388) only
   matches a bare `ref.func`/`ref.null` as the entry's own head atom; an
   entry spelled `(item (ref.func $f))` or the flatter `(item ref.func
   $f)` (a folded instruction with its own operand written inline inside
   `item`, per the grammar's `elemexpr ::= "(item" expr ")"` production)
   falls into the `_ =>` catch-all, producing `"expected (ref.func ...)
   or (ref.null ...), found list"`. Confirmed live in `elem.wast` itself,
   line 11: `(elem funcref (ref.func $f) (item ref.func $f) (item
   (ref.null func)) (ref.func $g))` — this is the file's OWN first `elem`
   declaration (byte ~156, the very first NYS position this spec's own
   probe found in that file).
3. **A compound (non-bare-atom) reftype token is not recognized** by the
   reftype-vs-funcidx-list disambiguation step (`module.rs:2314-2322`,
   which only checks `.as_atom() == "funcref" || "externref"`). `elem.
   wast` line 576 uses `(elem declare (ref func) (ref.func 0))` — `(ref
   func)`, a non-null `funcref`, written as the full compound reftype
   form rather than the abbreviated `funcref` keyword. Falling through
   the atom check with `use_exprs = false`, this currently mis-parses as
   a funcidx-list whose sole "entry" is the list `(ref.func 0)`, and
   fails in `resolve_idx`'s generic non-atom branch.
4. **The inline table+elem shorthand** (`(table $t funcref (elem
   (ref.func $f) (ref.null func) (ref.func $g)))`, confirmed live in the
   corpus) is built by a SEPARATE function, `build_table_limits_and_
   elements` (`module.rs:2123-2158`), which has no exprs-list branch at
   all — it unconditionally calls `resolve_idx(&ctx.func_names, f,
   "func")` on every inline element (line 2135), so an inline exprs-list
   entry hits the exact same `resolve_idx`-non-atom failure as item 3.

None of items 1-4 touch `wasm_types::Element`'s representation — see
"Design" below for why.

### `wasm_types::Element`: no representation change needed

`code/packages/rust/wasm-types/src/lib.rs`, `Element` (lines 1354-1381),
confirmed by direct read:

```rust
pub struct Element {
    pub table_index: u32,
    pub offset_expr: Vec<u8>,
    /// `Some(idx)` for a real `ref.func idx` entry (or a bare
    /// funcidx-list entry, binary modes 0-3); `None` for a `ref.null`
    /// entry (binary exprs-list modes 4-7, task #97) -- an explicit null
    /// table slot, not merely absent.
    pub function_indices: Vec<Option<u32>>,
    pub is_passive: bool,
}
```

The doc comment on `function_indices` ALREADY documents "binary exprs-
list modes 4-7" as a case it's designed to hold — a previous pass
anticipated this exact gap and shaped the representation for it, without
finishing the decoder. Confirmed by direct read of every consumer this
spec's own scope touches:

- `code/packages/rust/wasm-runtime/src/lib.rs:2521-2523` (active-segment
  application in `instantiate()`): `table.set((offset_num + j) as u32,
  func_idx.map(TableElement::Raw))` where `func_idx: Option<u32>` comes
  straight from `elem.function_indices`. A `None` (from a `ref.null`
  entry) already produces a null table slot exactly as W35's
  `TableElement`/`resolve_function_ref_for_dispatch` machinery expects;
  a `Some(idx)` already produces an UNRESOLVED `TableElement::Raw(idx)`,
  resolved lazily at the eventual `call_indirect`/`table.get` read site
  through the SAME W35 machinery every other funcref-bearing table entry
  already goes through. Nothing here changes.
- `code/packages/rust/wasm-runtime/src/lib.rs:3120`:
  `engine.set_elements(instance.module.elements.iter().map(|elem|
  elem.function_indices.clone()).collect())` — feeds `wasm-execution`'s
  passive-segment storage (for `table.init`) the same `Vec<Option<u32>>`
  shape it already expects.
- `code/packages/rust/wasm-execution/src/lib.rs:6400` (the `table.init`
  opcode handler, `0xFC 0x0C`): `table.set((dest + i) as u32,
  segment[src + i].map(TableElement::Raw))` — same shape, same "`None`
  becomes null, `Some(idx)` becomes an unresolved raw handle resolved
  lazily" pattern.

**Conclusion**: because this repo's `read_elem_expr_entry`/`resolve_elem_
expr_entry` are already scoped to exactly the two shapes the corpus
needs (`ref.func` → `Some(idx)`, `ref.null` → `None`), and because
`Option<u32>` already round-trips through every consumer exactly as a
funcidx-list entry would, **the whole fix is confined to the two
PARSERS' own decoding logic** (`wasm-module-parser`'s binary flags,
`wasm-wast-parser`'s text-format acceptance) — nothing in `wasm-types`,
`wasm-runtime`, or `wasm-execution` needs to change. This is the one
place this spec's own scope is SMALLER than a first read of the addendum
would suggest, mirroring (in miniature, and in the opposite direction)
how W35 found the addendum's own suggested design didn't work as stated
— here, the existing design already works, once the parsers are taught
to reach it.

## Confirmed corpus impact (re-scoped, after the correction above)

Direct probe, current `main` tip, same throwaway-probe method throughout:

| File | Confirmed exprs-list-caused NYS | Other causes present (out of this spec's scope) |
|---|---|---|
| `elem.wast` | The file's dominant remaining cause — active-exprs-list rejection (several groups, up to 3 directives per failing module) plus binary flags 3/4/6/7 (4 distinct `module binary` fixtures) plus the `(item ...)`/`(ref func)` compound-reftype gaps (2 more) — roughly half of its 67 total NYS | `spectest.table`/`spectest.global_i32` import stubs (12, separate harness gap, Addendum 2 item 4); "table with an explicit init expression (function-references proposal)" (6, a genuinely different feature — table DECLARATIONS with an inline init expression, not element segments); `unknown table identifier "$e"` (4, undetermined, possibly a downstream symptom — re-probe after this spec's fix lands); 3 `no instruction-level type-checker` (pre-existing, unrelated) |
| `global.wast` | 6 (active-exprs-list rejection: `(elem (table $t) (global.get $g3) funcref (global.get $g4)) ...`-shaped fixtures) | 64 `spectest.global_i32` import-stub gap (Addendum 2 item 4, dominant cause) |
| `ref_func.wast` | 1 (active-exprs-list rejection) | 2 pre-existing `no instruction-level type-checker` |
| `table_init.wast`/`table_init64.wast` | 0 confirmed from exprs-list proper; 2 each from a GC-specific `arrayref`+`array.new_default` exprs entry, explicitly out of this spec's scope (see below) | 495/497 from the `table.init`/`table.copy` arity bug (see correction above) |
| `table_copy.wast`/`table_copy64.wast`/`bulk.wast` | 0 | Entirely the arity bug (see correction above) |
| GC-proposal remainder (`ref_eq.wast`, `ref_test.wast`, `ref_cast.wast`, `i31.wast`, `br_on_cast.wast`, `array.wast`, `struct.wast`, `type-subtyping.wast`, `type-rec.wast`, `table-sub.wast`, `ref.wast`, `call_ref.wast`, `return_call_ref.wast`, ~500 combined per Addendum 2) | A small handful at most (`table-sub.wast` shows one hit; `ref_func.wast` above) | **Dominated by a separate, much larger, unrelated gap**: table DECLARATIONS in this repo only accept `funcref`/`externref` as their element type (`wasm-wast-parser/src/module.rs:1343-1352`, `2057`, `2118` all reject any other reftype keyword with `"expected funcref or externref, found ..."`) — `anyref`/`i31ref`/`structref`/`eqref`-typed tables are rejected before element-segment parsing is ever reached. Confirmed by direct probe of every file in this bucket: the overwhelming majority of their NYS entries cite this exact table-declaration-reftype rejection, not anything about element-segment content. Addendum 2's own hedge ("unconfirmed... may substantially shrink... or may not") is resolved: it mostly does NOT shrink from this spec's fix. A full-GC-reftype-tables spec is a separate, large, future item, out of scope here. |
| `memory_copy0.wast`/`memory_copy1.wast` | 0 | Multiple-named-memories text-format gap (`(memory $mem3 (data ...))`-shaped, unrelated to element segments at all) |
| `select.wast` | 0 (confirmed: `select`'s own typed-result folded-form gap, Addendum 2 item 2, entirely unrelated) | — |

**Bottom line**: this spec's own fix is worth roughly 15-20 directives
directly (`elem.wast`, `global.wast`, `ref_func.wast`), not "2000+." It
remains worth doing — `elem.wast` is this campaign's own most-scrutinized
file (W35's closing addendum already got it to 18/19 real-failure
passing; this closes real, confirmed NYS entries in the SAME file) — but
sequencing decisions should be made on the corrected numbers above, not
Addendum 2's original estimate.

## Design

### 1. `wasm-module-parser`: binary decoder additions

Add flags `3`, `4`, `6`, `7` to `parse_element_section`'s `match flags`
(`lib.rs:1154-1198`), alongside the existing `0`/`1`/`2`/`5` arms. A
shared `read_reftype_byte` helper (new, small, private to this module)
backs both the extended flag-`5` arm and the three new exprs-list arms:

```rust
/// Reads one `reftype` byte sequence (binary §5.3.4), scoped to the
/// shapes this repo's own corpus actually uses: the two abbreviated
/// abstract forms (`0x70` funcref, `0x6F` externref), and the two
/// "concrete-heaptype" prefixes (`0x64`=non-null, `0x63`=nullable)
/// followed by the single abstract heap type byte `0x70`/`0x6F` (e.g.
/// `elem.wast`'s own `(ref func)` reftype, binary `\x64\x70`). Anything
/// else (a real GC concrete-type-index heap type, `0x71` anyref, etc.)
/// is a loud, clear rejection -- no corpus file needs it here, and
/// element-segment reftype checking has no bearing on whether this
/// repo's other GC-reftype-table gaps (see this spec's own corpus-impact
/// table) get fixed.
fn read_reftype_byte(p: &mut Parser) -> Result<(), WasmParseError> { ... }
```

- Flag `3` (declarative, funcidx-list): read `elemkind` (must be `0x00`,
  same check flag `1` already makes), then the funcidx vec — IDENTICAL
  body to flag `1`'s arm, just `is_passive: true` (this repo's existing
  declarative-as-passive convention, see above) with no offset/table
  fields, same as flag `1` already sets.
- Flag `4` (active, implicit table `0`, exprs-list): read the offset
  `expr`, then `read_reftype_byte`, then the exprs vec via the EXISTING
  `read_elem_expr_entry` (unchanged) — `is_passive: false`,
  `table_index: 0`.
- Flag `6` (active, explicit table index, exprs-list): read `tableidx`,
  offset `expr`, `read_reftype_byte`, exprs vec — `is_passive: false`.
- Flag `7` (declarative, exprs-list): `read_reftype_byte`, exprs vec —
  `is_passive: true`, no offset/table fields (mirrors flag `3`).
- Flag `5`'s existing hardcoded `if reftype != 0x70 { error }` (line
  1190) is replaced by a call to the same `read_reftype_byte` helper, so
  passive exprs-list segments also accept `externref` (and the two
  concrete-heaptype-prefixed forms) — closing a latent asymmetry (a
  passive `externref` exprs-list segment would otherwise still fail even
  after this spec, despite active ones now working).

No change to `read_elem_expr_entry` itself — it already decodes exactly
the two entry shapes (`ref.func`/`ref.null`) every flag variant needs.

### 2. `wasm-wast-parser`: text-format grammar additions

Four changes to `code/packages/rust/wasm-wast-parser/src/module.rs`, all
localized to `build_elem`, `resolve_elem_expr_entry`, and `build_table_
limits_and_elements`:

1. **Delete the active+exprs-list rejection** (lines 2323-2343, including
   its now-superseded `/security-review`-citing comment) — replace with a
   comment explaining the ACTUAL current scope (funcref/externref/`(ref
   func)`-shaped reftypes only, matching `read_elem_expr_entry`'s binary-
   side scope) and pointing at this spec for the historical context of
   why the old rejection existed and why it's gone.
2. **Recognize a compound reftype token** in the disambiguation step
   (lines 2314-2322): in addition to the existing bare-atom check
   (`funcref`/`externref`), also accept a LIST whose head atom is `ref`
   followed by `func`/`null func`/`extern`/`null extern` (reusing
   whatever existing helper already parses `(ref ...)`/`(ref null ...)`
   value-type syntax elsewhere in this file for table/param/result types
   — do not hand-roll a second copy of that grammar). Sets `use_exprs =
   true` identically to the bare-`funcref`/`externref` case.
3. **Teach `resolve_elem_expr_entry` the `(item ...)` wrapper**: add a
   `Some("item")` arm before the existing `ref.func`/`ref.null`/catch-all
   match. Per the grammar, `item`'s content is itself a folded
   instruction, which the corpus writes either flattened (`(item ref.func
   $f)` — `items[1]` is the atom `"ref.func"`, the rest of `items[1..]` is
   its own operand) or fully nested (`(item (ref.null func))` —
   `items[1]` is itself a one-element list). Handle both by re-dispatching
   `items[1..]` (or, in the nested case, the single inner list) through
   the SAME match this function already has — no new logic, just an
   extra layer of unwrapping before it.
4. **Extend `build_table_limits_and_elements`'s inline shorthand**
   (`(table $t <reftype> (elem ...))`, lines 2123-2158) with the identical
   reftype-vs-funcidx-list disambiguation `build_elem` already has: if
   `elem_items[1]` is a bare `funcref`/`externref` atom or a compound
   `(ref ...)` list (per change 2 above), treat `elem_items[2..]` as
   exprs-list entries via `resolve_elem_expr_entry`; otherwise keep
   today's funcidx-list behavior unchanged. This is the ONLY site in this
   spec's scope that currently has ZERO exprs-list handling at all (every
   other site has partial support already).

None of these four changes touch `Element`'s construction beyond what
`build_elem`/`build_table_limits_and_elements` already do — every new
code path still produces the same `Vec<Option<u32>>` the existing
(passive) exprs-list path already produces.

### 3. Runtime wiring: no changes

As established in "What already exists" above, `wasm-runtime`'s active-
segment application loop and `wasm-execution`'s `table.init` opcode
handler already consume `Element::function_indices`/the passive-segment
storage generically as `Vec<Option<u32>>`, resolving each `Some(idx)`
through W35's `TableElement::Raw` → `resolve_function_ref_for_dispatch`
machinery and each `None` as a null table slot. An active segment
produced via the new binary flags 4/6 or the new text-format acceptance
flows through the EXACT SAME `instantiate()` loop
(`wasm-runtime/src/lib.rs:2441-2533`) an active funcidx-list segment
already does — `is_passive: false`, real `table_index`/`offset_expr`,
`function_indices` populated from exprs instead of bare funcidx atoms.
Nothing downstream can tell the difference, by design.

## Explicitly out of scope for this spec

- **The `table.init`/`table.copy` folded-form arity bug** (§"Correction"
  above) — real, high-leverage, but a wholly separate `wasm-wast-parser`
  bug unrelated to element-segment content. Listed as slice 0 below
  precisely because it should land first, not because it's part of this
  spec's own subject.
- **GC-reftype-typed tables** (`anyref`/`i31ref`/`structref`/`eqref`
  tables, and by extension any element segment targeting one) — a large,
  separate, pre-existing restriction in `wasm-wast-parser`'s table-type
  parsing (three call sites, cited above), confirmed to be the DOMINANT
  cause of the GC-proposal-remainder NYS bucket, not primarily gated by
  this spec's own exprs-list fix. Worth its own future census-and-spec
  pass; Addendum 2's "re-probe after this spec lands" hedge for that
  bucket is resolved by this spec's own corpus-impact table above, so a
  future session does not need to re-litigate it.
- **`arrayref`/other GC-reftype-valued exprs-list entries** (confirmed
  live in `table_init.wast`/`table_init64.wast`, 2 each: `(elem $elem
  arrayref (item (array.new_default $arr (i32.const 0))))`) and
  **`i31ref`-valued entries** (`(item (ref.i31 (i32.const 999)))`,
  confirmed live in `i31.wast`) — both need a GC-heap-typed
  `TableElement`/`Element` payload richer than "funcref-or-null," and
  both are blocked upstream anyway by the GC-reftype-tables gap just
  above (their target tables are `arrayref`/`i31ref`-typed, which cannot
  even be DECLARED yet). Not attempted here; `read_elem_expr_entry`/
  `resolve_elem_expr_entry` continue to reject anything other than
  `ref.func`/`ref.null`, loudly, exactly as today.
- **`assert_invalid`-driven semantic validation of exprs-list entries**
  — `elem.wast` has several `assert_invalid` cases specifically probing
  this feature once it parses (lines ~860-889: an `item` with two
  instructions, an `item` producing the wrong value type, an `item`
  containing a non-constant instruction like `call $f`), expecting
  rejection with `"type mismatch"`/`"constant expression required"`. This
  spec's own text-format changes will make these MODULES parse further
  than they do today (a `(item (ref.null func) (ref.null func))` two-
  instruction `item` needs its own explicit "`item` must contain
  EXACTLY one instruction" check, not left to fall out of existing
  machinery by accident) — whoever implements this spec should verify
  each such `assert_invalid` case still grades correctly (either a parse-
  time rejection or a NotYetSupported, per this harness's own established
  `assert_invalid`-without-a-type-checker convention — see `code/specs/
  W05-wasm-conformance-harness.md` §4.3), not silently start accepting a
  module the corpus expects rejected.
- **`elem.wast`'s `unknown table identifier "$e"` (4 NYS)** and **"table
  with an explicit init expression (function-references proposal)" (6
  NYS)** — the former undetermined (re-probe after this spec lands, it
  may or may not be a downstream symptom), the latter a genuinely
  different feature (table DECLARATIONS carrying an inline initializer
  expression, unrelated to element segments). Both left for a future
  pass.
- **The `spectest.table`/`spectest.global_i32` host-module import gap**
  (`elem.wast`, `global.wast`) — already scoped as Addendum 2's own item
  4, unrelated to this spec.

## Recommended slice decomposition

Following this campaign's own established discipline (dependency-ordered,
each independently verifiable against a real corpus delta):

0. **(Not part of this spec's own subject, but should land first for
   leverage reasons.)** Fix `encode_table_init_flat`/`encode_table_copy_
   flat`'s folded-form arity detection (§"Correction" above). Verify:
   re-run `wasm_conformance_report --write-baseline`; expect
   `table_copy.wast`/`table_copy64.wast` to move close to fully passing
   (566/566 NYS → mostly `Pass`, modulo any unrelated real failures this
   uncovers once the modules actually build) and `table_init.wast`/
   `table_init64.wast`/`bulk.wast` similarly, MINUS the 2-each `arrayref`
   exprs-list cases this spec explicitly does not attempt. This is by far
   the single highest-leverage item in the current backlog — do it before
   anything below, and do not let its own corpus delta get tangled up
   with slices 1-4's verification.
1. **`wasm-module-parser` binary decoder**: `read_reftype_byte`, flags
   `3`/`4`/`6`/`7`, flag `5`'s generalization. Verify: `cargo test -p
   wasm-module-parser` unchanged pass rate plus new unit tests for each
   of the 4 new flag values (round-trip a hand-built binary fixture per
   flag, mirroring this crate's own existing per-flag test style, e.g.
   `test_element_section` at `lib.rs:2363`); no corpus-wide run needed yet
   (nothing in `wasm-wast-parser`/`wasm-conformance` can PRODUCE these
   flags as output until slice 2, but the embedded `(module binary ...)`
   fixtures in `elem.wast` exercise the DECODER directly and should be
   spot-checked against this slice alone via a throwaway probe, the same
   method this spec's own corpus-impact table used).
2. **`wasm-wast-parser` text-format grammar**: the four `module.rs`
   changes in "Design" §2, in the order listed (deleting the rejection
   is safe only once the OTHER three changes give the accepted forms
   somewhere correct to go — recommended sub-order: 3 (`item` wrapper)
   and 2 (compound reftype) first, since both are needed for existing
   PASSIVE forms too and are independently testable against today's
   passive-only acceptance; then 1 (delete the active rejection); then 4
   (inline shorthand), which is the most independent of the four. Verify:
   `cargo test -p wasm-wast-parser`; new unit tests for each of the four
   corpus-confirmed shapes (`elem.wast` lines 11, 43, 576, and the inline
   `(table $t funcref (elem ...))` fixture).
3. **Full corpus re-verification.** Re-run `wasm_conformance_report
   --write-baseline` and diff programmatically against the post-slice-0
   baseline. Expect (per the corrected corpus-impact table above):
   `elem.wast`'s NYS count to drop by roughly half; `global.wast` NYS to
   drop by 6; `ref_func.wast` NYS to drop by 1. Expect NO other file's
   tally to move from slices 1-2 alone. Then, per Addendum 2's own
   "re-probe before scoping further" discipline, re-probe the GC-
   proposal-remainder bucket, `bulk.wast` (already resolved by slice 0,
   should show 0 residual exprs-list-attributable NYS), and `imports*.
   wast` — this spec's own corpus-impact table already did this
   re-probing for the GC-remainder bucket and found it mostly does NOT
   shrink (dominated by the separate reftype-tables gap), so a future
   session does not need to redo that specific check, only confirm it
   didn't regress.

## Verification plan (for whatever session implements this)

1. `cargo build --workspace` and `cargo test --workspace` (or at minimum
   `-p wasm-module-parser -p wasm-wast-parser -p wasm-runtime -p
   wasm-execution -p wasm-conformance`) clean, after EACH slice, not just
   at the end — per this campaign's own "verify the narrow piece before
   attempting the wiring" precedent (W35 §"Recommended slice
   decomposition").
2. `cargo run --release --bin wasm_conformance_report -p wasm-conformance
   -- --write-baseline`, diffed against the pre-slice-0 baseline,
   confirming exactly the deltas predicted per slice above and no
   regressions anywhere else in the 257-file corpus.
3. For every corpus-cited fixture in this spec (`elem.wast` lines 11, 43,
   576, 585 [binary], 869-889 [`assert_invalid`], `global.wast`'s
   `funcref`-exprs `elem` fixtures, the inline `(table ... (elem ...))`
   shorthand), spot-check with a direct `run_wast_source` probe (the same
   throwaway-bin method this spec's own investigation used) rather than
   trusting the aggregate report number alone — the aggregate can hide a
   fixture that now parses but produces the WRONG table contents (e.g. a
   `ref.null` silently becoming function index `0` instead of a true null,
   the exact failure mode `build_elem`'s own now-deleted `/security-
   review` comment warned about for the ENCODER side — verify the
   DECODER side has no analogous silent-wrong-value failure mode either).
4. Confirm every `assert_invalid` case newly reachable by this spec's
   text-format changes (§"Explicitly out of scope," bullet on semantic
   validation) grades as `NotYetSupported` or a genuine parse-time
   rejection — never as a false `Pass` from a module that should have
   been rejected, and never as a new `Fail` this campaign would have to
   separately track down.
