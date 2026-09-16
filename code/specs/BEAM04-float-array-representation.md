# BEAM04 — float-array representation for the BEAM backend

**Status:** Delivered — this slice.
**Depends on:** `BEAM03-float-lowering.md` (f64 scalar lowering — this spec
only adds f64-typed *arrays*, reusing BEAM03's `const`(f64)/arithmetic
lowering unchanged).
**Selected by:** `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`'s "VM-LOOP-24"
section, which found `iir-to-beam` represents every `alloc_array`/
`array_set`/`array_get` with Erlang's `:atomics` module, and `:atomics` can
only hold 64-bit INTEGERS — every BASIC numeric-array write traps
(`{badarg,[{atomics,put,[Ref,Index,FloatValue],...}]}`) at runtime, even
though the program compiles and validates cleanly.

## 1. Problem

BASIC arrays are `array<f64>`: BA7-1b routes every scalar numeric value
through the shared f64 track, so even `DIM A(3)` (a plain numeric array with
no fractional literals anywhere in the program) allocates and mutates an
`array<f64>`. `iir-to-beam/src/lower.rs`'s existing `alloc_array`/
`array_set`/`array_get` lowering (see its own "Mutable memory: the `:atomics`
module" comment) represents EVERY array — regardless of element type — as an
Erlang `atomics` reference: `atomics:new(N, [])` to allocate, `atomics:put/3`
to write, `atomics:get/2` to read. `atomics` is a fixed-size, off-heap array
of 64-bit integers with destructive O(1) `get`/`put` — and, confirmed
directly on real `erl`, integers ONLY:

```erlang
1> R = atomics:new(4, []).
2> atomics:put(R, 1, 40).      % ok — integer
ok
3> atomics:put(R, 1, 40.0).    % float — traps
** exception error: bad argument
     in function  atomics:put/3
        called as atomics:put(#Ref<...>,1,40.0)
```

So every `array_set` on a BASIC numeric array traps, unconditionally. This
blocked 4 Dartmouth BASIC corpus rows (1-D array, 2-D array, `DATA`/`READ`/
`RESTORE`, and fractional `DATA`) per VM-LOOP-24's probe findings. Two more
rows (string-typed arrays and mixed numeric/string `DATA`) fail validation
outright for an unrelated reason (`iir-to-beam` has no `str`-typed array
element representation at all) — see §5, explicitly out of scope here.

## 2. Research method: two concrete candidates, tested against real `erl`

Per this backlog's "probe before declaring" discipline, both directions the
backlog named were implemented as small standalone `.erl` files, compiled
with the real, locally-installed `erl`/`erlc` (OTP 29; CI pins 27.3.4.11 —
see BEAM03 §3.3 for why that split matters and how this repo already
handles it), and inspected with `erlc -S` disassembly — not chosen by
intuition.

### 2.1 Candidate A: bit-reinterpret the f64 as an i64, keep `:atomics`

**Does the bit pattern round-trip exactly?** Yes, confirmed for a wide set of
finite values including `0.0`, `-0.0`, subnormal-adjacent magnitudes,
`±1.7976931348623157e308` (max finite), and `2.2250738585072014e-308` (min
normal):

```erlang
lists:foreach(fun(F) ->
    <<I:64/integer-unsigned-big>> = <<F:64/float-big>>,
    <<F2:64/float-big>> = <<I:64/integer-unsigned-big>>,
    true = (F =:= F2)
end, [0.0, -0.0, 1.0, -1.0, 3.14, -3.14, 1.0e300, 1.0e-300,
      123456789.123456, 2.2250738585072014e-308, 1.7976931348623157e308]).
```

Every case matched exactly (`match=true`). So the REPRESENTATION question has
a clean answer: an f64's raw IEEE-754 bit pattern, reinterpreted as an
unsigned 64-bit integer, is exactly what `atomics:put/3` already accepts,
and reversing the reinterpretation on read recovers the original float
bit-for-bit.

**What would the actual BEAM instruction sequence look like?** Disassembling
the minimal round-trip functions with `erlc -S`:

```erlang
to_bits(F) ->
    <<I:64/integer-unsigned-big>> = <<F:64/float-big>>,
    I.

to_float(I) ->
    <<F:64/float-big>> = <<I:64/integer-unsigned-big>>,
    F.
```

disassembles to (excerpted):

```
{function, to_bits, 1, 2}.
    {bs_create_bin,{f,0},0,1,64,{x,0},
                   {list,[{atom,float},1,1,nil,{x,0},{integer,64}]}}.
    {bs_start_match4,{atom,no_fail},1,{x,0},{x,1}}.
    {bs_match,{f,3},{x,1},
              {commands,[{ensure_exactly,64},
                         {integer,2,{literal,[]},64,1,{x,2}}]}}.
    {move,{x,2},{x,0}}.
    return.

{function, to_float, 1, 5}.
    {bs_create_bin,{f,0},0,1,64,{x,0},
                   {list,[{atom,integer},1,1,nil,{x,0},{integer,64}]}}.
    {bs_start_match4,{atom,no_fail},1,{x,0},{x,1}}.
    {test,bs_get_float2,{f,6},2,
          [{tr,{x,1},{t_bs_context,64}},{integer,64},1,
           {field_flags,[...,unsigned,big]}],{x,2}}.
    {bs_match,{f,6},{x,1},{commands,[{ensure_exactly,0}]}}.
    {move,{x,2},{x,0}}.
    return.
```

This needs THREE opcode families `iir-to-beam`/`ir-to-beam` have never
implemented for any op today: `bs_create_bin` (construction), `bs_start_match4`
+ `bs_match` (matching, with a nested `{commands, [...]}` operand list whose
shape is unlike every other operand this encoder handles), and `test
bs_get_float2` (a typed extraction inside the match). Each of these has
materially more complex operand encoding than anything currently
implemented — `gc_bif1`/`gc_bif2`/`call_ext` all take a flat, fixed-shape
operand list; `bs_match`'s `commands` list is itself a nested, variable-length
sub-structure requiring new encoding logic in `ir-to-beam`'s compact-term
encoder, not just a new opcode constant.

### 2.2 Candidate B: switch float-element arrays to `:ets`

`:ets` (Erlang Term Storage) is a mutable table that stores arbitrary Erlang
terms — including floats — natively, no bit-reinterpretation needed.
Disassembling a minimal `:ets`-backed array helper:

```erlang
new_table() -> ets:new(myarr, [set, public]).

put_val(Tab, Idx, Val) ->
    Tup = erlang:list_to_tuple([Idx, Val]),
    ets:insert(Tab, Tup),
    ok.

get_val(Tab, Idx) -> ets:lookup_element(Tab, Idx, 2).
```

disassembles to (excerpted):

```
{function, put_val, 3, 4}.
    {put_list,{x,2},nil,{x,0}}.
    {put_list,{x,1},{x,0},{x,0}}.
    {call_ext,1,{extfunc,erlang,list_to_tuple,1}}.
    ...
    {call_ext,2,{extfunc,ets,insert,2}}.
    ...

{function, get_val, 2, 6}.
    {move,{integer,2},{x,2}}.
    {call_ext_only,3,{extfunc,ets,lookup_element,3}}.
```

Every operation here is either `put_list` (already implemented — the exact
cons-cell construction `call_closure`'s arg-list building already uses, per
LANG35) or `call_ext`/`call_ext_only` against an ordinary library function
(the exact shape `math:pow/2`, `math:sqrt/1`, `lists:sublist/3`, etc. already
use, per BEAM03/VM-LOOP-24). **Zero new BEAM opcodes.**

Two further properties were verified functionally (not just by disassembly)
on real `erl`, using dynamic (non-compile-time-constant) values:

- **`ets:new/2` needs no size argument.** Unlike `atomics:new(N, [])`, `ets`
  tables grow dynamically — `alloc_array`'s length source is simply unused
  on this path.
- **`ets:new/2` returns a fresh, private table identifier on every call, even
  when the same name atom is reused and `named_table` is never passed** —
  confirmed with `ets:new(arr, [])` called twice, giving two independent,
  non-colliding tables. So a fixed atom (`farray`) is safe to reuse across
  every `alloc_array` call site in a module; concurrently DIMmed arrays never
  alias.
- **`ets:new(Name, [])` defaults to `type == set, protection == protected`**,
  which is sufficient for this backend's single-process execution model —
  confirmed via `ets:info/1`.
- **Overwrite semantics match `atomics:put`:** re-inserting at an existing
  key replaces the value (confirmed: insert `{0, 1.0}` then `{0, 99.5}`,
  `ets:lookup_element` then returns `99.5`).
- **Missing-key read traps cleanly:** `ets:lookup_element(Tab, MissingKey, 2)`
  raises `badarg` — the same trap-on-out-of-range failure mode
  `atomics:get` already has for out-of-bounds `array<i64>` reads. (See §4 for
  the one place this trap-on-miss behavior diverges from `atomics`'
  zero-initialization.)

## 3. Decision: `:ets`, not bit-reinterpretation

**`:ets` was chosen.** The backlog's own framing suggested reusing `:atomics`
"is likely simpler... since it wouldn't need a new table-lifecycle model" —
that assumption does NOT hold up under measurement. `:ets` needs no new
BEAM opcodes at all (reusing `put_list` and `call_ext`, both already fully
implemented and exercised by dozens of existing tests), while
bit-reinterpretation, despite round-tripping perfectly at the VALUE level,
needs three entirely new, structurally more complex opcode families
(`bs_create_bin`, `bs_start_match4`/`bs_match` with a nested `commands`
operand shape, and a typed `test bs_get_float2` extraction) that this
backend's encoder has never needed for anything else. The `:ets`
"table-lifecycle" concern the backlog raised turned out to be a non-issue in
practice: no explicit per-array cleanup is needed (see §4), and table
creation is a single `call_ext` exactly like `atomics:new`.

This also keeps the change LOCAL: only `alloc_array`/`array_set`/
`array_get` with `type_hint == "array<f64>"`/`"f64"` dispatch to `:ets`;
every other array/tape use (Brainfuck's byte tape, the GOSUB return-address
`array<i64>` stack, the BASIC `DATA` pool's kind array) is untouched and
keeps using `:atomics` exactly as before. `add`/`sub`/`mul`/`cmp_*`/
`int_to_real`/etc. need zero changes — once a valid boxed float is in a
register (either from BEAM03's `const`(f64) or from this slice's
`ets:lookup_element`), the existing scalar f64 lowering already handles it.

## 4. Design: what ships in this slice

All changes are in `iir-to-beam/src/lower.rs`; `ir-to-beam` (the encoder) is
untouched — no new opcodes are needed.

- New atoms/imports, registered unconditionally at module setup (mirroring
  the existing "pre-register so indices are stable" pattern used for
  `math:*`/`atomics:*`): `ets:new/2`, `ets:insert/2`,
  `ets:lookup_element/3`, `erlang:list_to_tuple/1`, and a fixed atom
  (`farray`) used as every `:ets`-backed array's table-name argument (never
  passed with `named_table`, so it never causes a collision — see §2.2).
- `alloc_array` with `type_hint == "array<f64>"`: `x0 = 'farray' ; x1 = [] ;
  call_ext 2 ets:new/2 ; dest = x0`. The length source is validated for
  shape (still exactly one source operand) but its VALUE is unused — `:ets`
  tables grow dynamically. Every other `alloc_array`/`alloc_bytes` case is
  unchanged (still `:atomics`).
- `array_set` with `type_hint == "f64"`: stage `Ref`/`Idx`/`Val` into
  scratch registers above `meta.next_reg` (same parallel-move-hazard
  discipline as the existing `:atomics` `store_byte`/`array_set` staging),
  build `[Idx, Val]` right-to-left with `put_list` (the same pattern
  `call_closure`'s arg-list construction uses), convert to a tuple with
  `erlang:list_to_tuple/1` (`call_ext`), then `ets:insert(Tab, Tuple)`
  (`call_ext`). No index `+1`: unlike `:atomics`, `:ets` keys are used as
  given — it is not 1-indexed.
- `array_get` with `type_hint == "f64"`: `ets:lookup_element(Tab, Idx, 2)`
  (`call_ext`) returns the value directly — no list/tuple destructuring
  needed, unlike a naive `ets:lookup` + pattern-match. `2` is a literal
  (tuple position 2 = the `Val` half of `{Idx, Val}`).
- Both `array_set` and `array_get`'s new branches use the existing
  `save_live_across_imported_call!`/`restore_live_across_imported_call!`
  macros around their `call_ext` sequences (`alloc_array`/`array_set`/
  `array_get` were already in the `live_across` match list before this
  slice, since the `:atomics` path already emits `call_ext`, so no change
  was needed there). A scratch register staged above `meta.next_reg`
  surviving an intermediate `call_ext` before being used by a LATER call in
  the same instruction's lowering (`array_set`'s `Ref` surviving the
  `list_to_tuple` call before being used by `ets:insert`) is the same
  pattern `call_closure`'s existing, tested `r0`/`r3` staging already
  relies on.

### Known, documented limitation (not fixed, not exercised by any promoted row)

Unlike `atomics:new(N, [])`, which zero-initializes every cell, `ets:new`
does not pre-populate any rows. Reading an array index that was never
`array_set` therefore raises `badarg` (a clean trap) instead of returning
`0.0` the way an unwritten `atomics`-backed cell would. No promoted corpus
row exercises this — every promoted float-array row writes every cell it
later reads (confirmed by re-reading each row's source before promotion, not
assumed) — so this is left as a known, documented divergence rather than
guessed at or silently papered over, matching BEAM03's own precedent for
undecided platform questions (§6 there). A future slice that needs
zero-initialized reads would need either a bounded pre-fill loop at
`alloc_array` time (for a compile-time-constant N) or a design decision for
the general dynamic-N case; neither was needed by anything in this slice's
promoted scope.

## 5. Explicitly out of scope for this slice

- **String-typed arrays and mixed numeric/string `DATA`** (2 Dartmouth BASIC
  corpus rows: `10 DIM A$(2)...` and `10 DIM S$(1)\n20 DATA 20, "O", 22,
  "K"...`). These fail `iir-to-beam` VALIDATION outright
  (`UnsupportedType`: `type_hint == "str"` on `array_set`/`array_get`),
  independent of the float-storage question this slice closes —
  `iir-to-beam` has no `str`-typed array element representation at all yet.
  This is a separate, likely larger design item (it needs a BEAM
  representation for a *heterogeneous* element type, not just a wider
  scalar) and is untouched here.
- **`RND`** — still traps with `{badarith,[{erlang,'*',[undefined,...
  a module-global `erlang:get/1` read returning `undefined`. This is the
  same "RND's full DEF-FN-and-module-global chain" open design question
  flagged at VM-018, unaffected by array representation, and untouched here.
- **VM-060b (BEAM host input)** — the 5 `INPUT` Dartmouth BASIC rows remain
  blocked on the unscoped BEAM host-input design, unrelated to arrays.
- **`f32`** — this backend only ever sees `f64` from any current frontend.
- **The zero-initialization gap** documented in §4 above.

## 6. Validation

- `iir-to-beam` 0.12.0 → 0.13.0: 4 new tests (104 unit/integration tests + 5
  doc tests total, up from 100):
  - `test_94_f64_array_ops_use_ets_not_atomics` (unit/instruction-shape):
    confirms the emitted `call_ext`s for an all-f64-array module target
    `ets:new/2`/`ets:insert/2`/`ets:lookup_element/3`/
    `erlang:list_to_tuple/1` and NONE target `atomics:new/2`/`put/3`/
    `get/2` — checking the actual `call_ext` OPERANDS emitted, not just
    import-table presence (every import is pre-registered unconditionally
    at module setup, so import-table presence alone cannot distinguish
    "this module calls atomics" from "the backend always reserves the
    slot").
  - `test_95_real_erl_float_array_set_get_roundtrip` (real `erl`): the exact
    shape of the promoted 1-D-array corpus row — `alloc_array`, two
    `array_set`s, two `array_get`s, `add` — executed on real Erlang,
    expecting `42.0`.
  - `test_96_real_erl_float_array_overwrite` (real `erl`): writes index 0
    twice (`1.0` then `99.5`) and confirms the read-back value is `99.5`,
    proving `ets:insert` overwrites rather than duplicates.
  - `test_97_real_erl_float_array_unset_read_traps` (real `erl`): pins the
    §4 known limitation as an intentional, tested trap rather than an
    unnoticed regression risk — reading a never-`array_set` index raises
    `badarg` (confirmed via nonzero exit + stderr containing `"badarg"`).
  - All-target Clippy with warnings denied is clean.
- `lang-aot` 0.340.0 → 0.341.0: promoted the 4 Dartmouth BASIC `lang_matrix`
  rows VM-LOOP-24 found blocked on this exact gap — the 1-D array row, the
  2-D array row, the `DATA`/`READ`/`RESTORE` row, and the fractional-`DATA`
  row — via a new dedicated
  `portable_text_stdout_dartmouth_basic_beam_arrays_and_data` test (4
  programs) executed against real `erl` before promotion, per this
  backlog's "probe before declaring" gate.
  `feature_coverage_doc_counts_match_programs_source` was updated
  (Dartmouth BASIC tuple `(51, 396)` → `(51, 400)`) and passes against the
  live `PROGRAMS` corpus; `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth BASIC
  row, grand-total prose (1632 → 1636), and the "Implemented feature
  families" narrative row were all updated to match. No full
  `non_algol_matrix_every_proven_cell_agrees` capstone rerun is claimed for
  this slice, matching every prior BEAM03/VM-LOOP-24 slice's own precedent —
  the dedicated arrays/DATA test plus the full `iir-to-beam` suite
  (including the new real-`erl` tests) are the executed evidence.

Dartmouth BASIC now declares 43/51 rows on `Beam` — up from 39/51. Only 8
rows remain undeclared: the 5 `INPUT` rows (VM-060b), the 2 string-array/
mixed-`DATA` rows (§5), and `RND` (VM-018). Reprioritize after this merges:
VM-060b (BEAM host input) is now the single highest-value remaining
non-ALGOL BEAM item for Dartmouth BASIC, unblocking 5 rows at once; the
string-array representation gap is the next-largest design item (2 rows,
but architecturally bigger than the float-storage question this slice
closed); `RND` is a single row gated on a design question unrelated to
arrays or input. VM-041 (Twig dynamic-string/record/closure BEAM isolation)
remains the other standing non-ALGOL candidate untouched by this slice.
