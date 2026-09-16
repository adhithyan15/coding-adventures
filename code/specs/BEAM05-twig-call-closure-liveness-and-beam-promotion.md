# BEAM05 — Twig `call_closure` liveness fix and BEAM probe-first promotion

**Status:** Delivered — first cut (§1–§5) plus a direct follow-up (§6) that
closes the `match`/`union` gap §3.2/§5 left deferred. Twig is now **49/49**
on `Beam`.
**Depends on:** `BEAM03-float-lowering.md` (f64 scalar lowering — unrelated
but shares the `call_ext` liveness discipline this spec extends),
`BEAM04-float-array-representation.md` (the immediately prior slice in this
backlog).
**Selected by:** `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`'s "BEAM04"
section, which named VM-041 (Twig BEAM closure/string/record isolation) as
the last standing non-ALGOL BEAM item with a large undeclared surface
(20/49 rows), and an orchestrating session's own direct code read, which
independently confirmed a dormant liveness bug in `call_closure`'s lowering
that had to be fixed before any closure row could safely be promoted.

## 1. Problem

Two problems, addressed together because the second cannot be safely
attempted without the first:

1. **A confirmed silent-data-corruption bug (VM-D035).** `iir-to-beam`'s
   `"call_closure"` lowering arm (`src/lower.rs`) emits TWO `OP_CALL_EXT`
   instructions and wraps NEITHER in the liveness save/restore macros every
   other multi-call arm in this file uses — even though `call_closure` was
   already listed in the `live_across` liveness match. This is the exact bug
   class VM-D029 already fixed once for six `:atomics` ops: a call clobbers
   every X-register, so any variable live across an unwrapped call has its
   value silently overwritten instead of the compiler raising an error.
2. **Twig's remaining 29 non-`Beam` `lang_matrix.rs` rows had never been
   probed against real `erl`, individually, since the frontend's dynamic
   arithmetic/list-op/string/closure lowering passes were written.** The
   backlog's prior framing ("dynamic-string/record/closure BEAM isolation")
   assumed this was a large, unscoped design item. It was not established
   which of the 29 rows actually need new capability versus which simply had
   never been run.

## 2. The call_closure liveness fix (VM-D035)

### 2.1 What the lowering does

`call_closure(handle, arg0, arg1, …)` dispatches a Twig closure — a cons
cell `(fn_atom . captures)` built by `alloc_closure` — via Erlang's
`erlang:apply/3`:

```text
get_list  {x, handle}  {x, r0}  {x, r1}   % r0=fn_atom, r1=caps
move      {a, nil}     {x, r2}             % r2 = nil
put_list  {x, argN}    {x, r2}  {x, r2}   % build args from right, into r2
…
move      {x, r1}      {x, 0}              % x0 = caps
move      {x, r2}      {x, 1}              % x1 = args
call_ext  2  {u, import_append}            % x0 = caps ++ args
move      {x, 0}       {x, r3}             % r3 = combined
move      {a, mod_atom} {x, 0}             % x0 = module atom
move      {x, r0}      {x, 1}              % x1 = fn_atom
move      {x, r3}      {x, 2}              % x2 = combined
call_ext  3  {u, import_apply}             % x0 = result
move      {x, 0}       {x, r_dest}         % save result
```

Two `call_ext`s — `erlang:'++'/2` then `erlang:apply/3` — each of which
clobbers every X-register on real BEAM.

### 2.2 The gap

`live_across` (the per-function map from a call instruction's index to the
SSA variables that must survive it) already lists `"call_closure"` in its
match — a prior slice's comment there explicitly names the risk:

> `call_closure` emits TWO call_ext (erlang:'++'/2 then erlang:apply/3). It
> was missing here before the six memory ops were added, so a variable live
> across a closure call was silently destroyed.

But the `"call_closure"` lowering ARM itself never called
`save_live_across_imported_call!`/`restore_live_across_imported_call!` — the
liveness set was computed and then never read back out. This is the exact
"wrong VALUE, not a crash" failure mode VM-D029 already fixed once. It was
CURRENTLY DORMANT: confirmed by inspection that no `lang_matrix.rs` row
exercising `call_closure` declared `Beam` before this slice, so nothing in
CI had ever exercised the gap.

### 2.3 The fix

Both `call_ext` emissions are wrapped in a SINGLE
`save_live_across_imported_call!(cur_idx)` / `restore_live_across_imported_call!(cur_idx)`
pair spanning both calls. This mirrors the `array_set` (f64/ets, BEAM04) arm,
which already wraps its OWN two-call sequence (`erlang:list_to_tuple/1` then
`ets:insert/2`) in one such pair — `cur_idx` indexes a single `live_across`
entry per IIR instruction, and `call_closure` is one IIR instruction
regardless of how many BEAM calls it lowers to, so one save/restore pair is
correct and sufficient (not one per `call_ext`).

The call result is moved out of `x0` into `r_dest` BEFORE the restore runs,
mirroring `alloc_array`'s existing ordering: `restore_live_across_imported_call!`
moves saved Y-slots back into their original X registers, which could
include `x0` itself, so the freshly computed result must be captured first.

### 2.4 Proof

`iir-to-beam::test_98_real_erl_call_closure_survives_live_across_call`
builds:

```text
__add_fn(n: i64) : i64 = n + 0                 (identity, exercises add)

main() : i64
  k    = const 99 : i64      — allocated x0 (FIRST variable defined)
  arg  = const 7  : i64
  cl   = alloc_closure("__add_fn") : closure
  r    = call_closure(cl, arg) : any           — clobbers x0/x1/x2 internally
  sum  = add(r, k) : i64                       — k must still read 99 here
  ret sum : i64
```

`k` is deliberately allocated `x0` (the first-defined variable gets the
first register) — the EXACT register `call_closure`'s internal
`move {x,r1} {x,0}` (caps → x0) clobbers first. Without the fix, `k`'s
register is overwritten mid-dispatch and ends up holding the closure's own
result (`7`) instead of `99`, so `sum` comes out `14` instead of the correct
`106`. **Verified this is not a hypothetical:** reverted the fix, re-ran the
test, and confirmed it fails with exactly `14` (the predicted wrong value,
not a crash or a random garbage value) — then restored the fix and confirmed
the test passes again. This is the discipline "the test exercises the bug"
requires, distinct from merely "the test happens to pass."

## 3. Probe-first sweep of the remaining 29 Twig rows

Per this backlog's "probe before declaring, promote only proven cells"
discipline, a scratch probe (`code/packages/rust/lang-aot/tests/
scratch_twig_beam_probe.rs`, discarded before this PR — not part of the
permanent suite) compiled and ran EVERY one of the 29 not-yet-`Beam` Twig
`lang_matrix.rs` rows individually against real `erl`, unmodified from their
existing corpus source text, using `lang_aot::compile_source_to_beam` with
ONLY the capabilities that existed before this slice plus the §2 fix.

### 3.1 Findings: 27 of 29 passed unchanged

No new lowering, no new op, no new import was needed for any of these 27
rows — they simply had never been individually probed as corpus rows
before, the same "never actually run, not actually broken" shape
VM-LOOP-24 found for 12 Dartmouth BASIC rows:

- **Dynamic arithmetic / list ops (6 rows):** a bare literal (`42`);
  `(+ (car (cons 41 0)) 1)` (dynamic `any`-typed arithmetic — `unbox`/`add`/
  `box` — over a `car`'d cons cell); `(+ (length (list 1 2 3)) 39)`,
  `(list-ref (list 10 20 42) 2)`, `(cdr (assoc 2 …))`,
  `(null? (assoc 9 …))` (the `length`/`list-ref`/`assoc` synthesized
  recursive list-walk helpers from `iir-builtin-lowering::lower_list_ops`).
- **Closures (2 rows), unblocked directly by §2's fix:**
  `((lambda (x) (+ x 1)) 41)` (no-capture closure) and
  `(((lambda (x) (lambda (y) (+ x y))) 40) 2)` (a capturing closure
  returning another closure).
- **String ops (19 rows):** string literals, `let`/`let*` locals,
  non-escaping top-level `define`s, `substring`, `string<?`/`string>?`
  comparisons, the documented `string-ref` out-of-bounds trap (`Expect::
  Trap`), and a top-level function whose `str`-typed parameter is inferred
  four different ways (explicit `(s : str)` annotation, a literal direct
  call, a `str_concat`+`str_slice` actual, and a named/`let`/`let*` actual).
  `iir-to-beam`'s string primitives (`str_const`/`str_len`/`str_index`/
  `str_concat`/`str_slice`/`str_cmp`) were already proven on BEAM for COBOL-60
  and Dartmouth BASIC corpus rows — these 19 Twig rows exercise the exact
  same, already-implemented lowering.

Promoted via two new dedicated `lang_matrix.rs` tests, each program executed
against real `erl` before promotion:
`twig_beam_dynamic_arith_list_ops_and_closures` (8 programs) and
`twig_beam_string_ops` (19 programs).

### 3.2 Findings: 2 of 29 (`match`/`union`) remain deferred — a real, scoped gap

`(union Opt (Some (v : int)) (None)) (match (Some 42) ((Some v) v) ((None) 0))`
and its `None`-arm sibling both failed, in two stages:

**Stage 1 — a validator gap, found and fixed.** Compilation initially failed
validation:

```text
UnsupportedType: function "main", op "mov" has reference type "ref<LispyPair>";
heap pointer types are not supported in this BEAM backend (only ref<LispyPair>
on alloc/const, ref<any> on field_load, and ref<*> on ret are accepted)
```

Root cause: `twig-ir-compiler::compiler.rs`'s `emit_move` — which merges each
`if`/`match` arm's result into one mutable "phi" variable via a typed
`mov dst = src`, using the SOURCE value's own inferred type as the `mov`'s
type_hint — produces a `mov` with type_hint `ref<LispyPair>` whenever a
branch's value is a cons cell (here, a `union` variant constructor's
result). `iir-to-beam`'s validator rejected EVERY `mov` with any `ref<...>`
type_hint, but `lower.rs`'s `"mov"` arm already lowers `mov` correctly for
ANY type_hint — it is an unconditional `{operand} -> {x,rd}` BEAM `move`,
agnostic to what the register holds (confirmed by reading the arm: the only
operand-shape constraint is `integer_operand!`, which accepts a `Var` source
regardless of the variable's semantic type). The validator had simply never
been told to accept the ref-typed case, even though the analogous `"str"`
type_hint case was already accepted for `mov`.

Fixed by adding `"mov" => true` to the `ref<...>`-accepted-ops match in
`validate.rs`, mirroring the existing `"str"` exception. Proven CORRECT, not
just accepted, by a new dedicated test,
`test_99_real_erl_mov_ref_lispy_pair_lowers_correctly`: two cons cells
`(10 . nil)` and `(20 . nil)` are built in the two arms of an `if`, merged
into one `result` variable via the exact mutually-exclusive two-`mov`
pattern `emit_move` produces, and the head of whichever cell survived the
merge is read back — `cond = true` selects the first arm, so the correct
answer is `10`; a broken merge would read `20` or corrupt the register
entirely.

**Stage 2 — a separate, deeper, still-open gap.** With the validator fix
in place, compilation reaches LOWERING and fails there instead:

```text
UnsupportedOp { function: "Some", op: "field_store: found outside of
alloc+field_store+field_store pattern — lower alloc+2×field_store into
put_list before reaching the backend" }
```

`iir-to-beam`'s cons-cell construction fusion (`alloc` immediately followed
by exactly two `field_store`s → one `put_list`) only recognizes the THREE
instructions when they are textually adjacent (see `lower.rs`'s look-ahead
comment: "We peek at instructions [idx] and [idx+1]"). The synthesized
union-variant constructor function (`Some`, generated by Twig's union
lowering) interleaves a `mov` between the `alloc` and/or its `field_store`s
— exact position not yet pinned down further, since this slice deliberately
did not force a fix here.

**This is a genuine, scoped design/implementation gap, left deferred rather
than forced**, per this backlog's explicit "do not force a design decision"
instruction. The fix would be either (a) teaching the fusion look-ahead to
tolerate (or skip past) intervening non-`field_store` instructions between
`alloc` and its two `field_store`s, or (b) changing the union-variant
constructor's own codegen in `twig-ir-compiler` to avoid interleaving a
`mov` into that three-instruction window in the first place. Both are
real, bounded follow-up candidates for the NEXT VM-041 slice — neither was
attempted here, since diagnosing the exact interleaving point and choosing
between "backend tolerance" and "frontend codegen change" is itself real
research this slice's time budget did not cover, and the instruction that
selected this work explicitly said not to force it.

## 4. Contract (what changed)

- `iir-to-beam/src/lower.rs`: `"call_closure"` arm now wraps both `call_ext`
  emissions in one `save_live_across_imported_call!`/
  `restore_live_across_imported_call!` pair.
- `iir-to-beam/src/validate.rs`: `"mov"` added to the `ref<...>`-accepted-ops
  match (alongside `alloc`/`const`/`field_load`/`field_store`/`ret`).
- `iir-to-beam/tests/test_backend.rs`: `test_98_real_erl_call_closure_survives_live_across_call`,
  `test_99_real_erl_mov_ref_lispy_pair_lowers_correctly`.
- `lang-aot/tests/lang_matrix.rs`: 27 Twig rows promoted to declare `Beam`
  (20/49 → 47/49); two new dedicated tests
  (`twig_beam_dynamic_arith_list_ops_and_closures`, `twig_beam_string_ops`);
  `feature_coverage_doc_counts_match_programs_source`'s Twig tuple updated
  `(49, 363)` → `(49, 390)`.
- `code/specs/LANG-VM-FEATURE-COVERAGE.md`: Twig row and grand-total prose
  updated to match (363 → 390 Twig cells; 1636 → 1663 grand total).

## 5. Explicitly out of scope / deferred (at first-cut time — see §6, now closed)

- `match`/`union` (2 Twig rows) — the `alloc`+`field_store` fusion-adjacency
  gap described in §3.2. Not attempted at first-cut time. **Closed by the §6
  follow-up in this same spec** (same PR sequence): the exact interleaving
  was pinned down (a `box`, not a `mov`, between `alloc` and the first
  `field_store`) and fixed by reordering `twig-ir-compiler`'s codegen. Twig
  is now 49/49 on `Beam`.
- Twig's remaining non-BEAM gaps outside this backlog's non-ALGOL scope are
  unaffected (this slice touches only `iir-to-beam`/`lang_matrix.rs`
  BEAM-column declarations).
- No full `non_algol_matrix_every_proven_cell_agrees` capstone rerun is
  claimed, matching every prior BEAM03/VM-LOOP-24/BEAM04 slice's own
  precedent — the two dedicated tests plus the full `iir-to-beam` suite
  (including the two new real-`erl` tests) are the executed evidence.

## 6. Follow-up — pinning down and closing the `match`/`union` fusion gap

§3.2 left the exact interleaving point unpinned ("exact position not yet
pinned down further, since this slice deliberately did not force a fix
here"). This follow-up slice pins it down and closes it, per the same "probe
before declaring" discipline.

### 6.1 Pinning down the exact shape

Compiled the `match`/`union` corpus program
(`(union Opt (Some (v : int)) (None)) (match (Some 42) ((Some v) v) ((None) 0))`)
through `lang_aot::compile_source_to_iir` and dumped the synthesized `Some`
constructor function's instruction list directly (a scratch probe test,
`code/packages/rust/lang-aot/tests/scratch_union_probe.rs`, discarded before
this PR — not part of the permanent suite). The actual sequence, BEFORE this
follow-up's fix:

```text
[0] const  _nil1              : ref<LispyPair>   (= 0, the nil sentinel)
[1] alloc  _cell2              : ref<LispyPair>   (the field's cons cell)
[2] box    _fbox3 = box(v)     : ref<any>          <-- INTERLEAVED
[3] field_store _cell2, 0, _fbox3 : void
[4] field_store _cell2, 1, _nil1  : void
[5] const  _tag4  = 0          : i64               (the variant's tag)
[6] box    _tbox5 = box(_tag4) : ref<any>
[7] alloc  _head6              : ref<LispyPair>   (the tag/head cons cell)
[8] field_store _head6, 0, _tbox5 : void
[9] field_store _head6, 1, _cell2 : void
[10] ret   _head6              : ref<LispyPair>
```

This pins down BOTH open questions §3.2 left unresolved:

1. **The interleaved instruction is a `box`, not a `mov`** as §3.2's own
   note had guessed (that guess was written before this slice's direct
   inspection; the `mov`-related validator gap fixed in §3.2/§4 is real and
   separate — it fires on the `if`/`match` arm-merge phi, not here).
2. **It sits between `alloc` and the FIRST `field_store`** (index `[1]` and
   `[3]`), not between the two `field_store`s. The SECOND cons cell built by
   this same function — the tag/head cell at `[7]`–`[9]` — has NO
   interleaving at all: its `box` (`[6]`) already runs before its `alloc`
   (`[7]`), because `twig-ir-compiler::emit_union_def` happens to compute
   `tag_boxed` before `alloc_head` in source order. Only the PER-FIELD loop
   (`emit_union_def`'s `for field_name in params.iter().rev()`) has the
   `alloc`-before-`box` ordering that breaks fusion. (The record constructor,
   `emit_record_def`, stores fields raw/unboxed — no `box` at all — so it was
   never affected either.)

Root cause: `twig-ir-compiler::compiler.rs`'s `emit_union_def`, in its
per-field loop, computed `alloc` (the cons cell), THEN `box` (E6d-6b: boxing
the field value so `match`'s `field_load` + `unbox` round-trips on the tagged
backends), THEN the two `field_store`s. `iir-to-beam`'s fusion look-ahead
(`lower.rs`'s `"alloc" if instr.type_hint == "ref<LispyPair>"` arm) peeks
ONLY at `func.instructions[instr_idx]` and `[instr_idx + 1]` — the next two
instructions after the `alloc`, unconditionally — so an intervening `box`
instruction is read as if it were the first `field_store`, its `op` field
compared against `"field_store"` and found not to match, and the whole
lookahead falls through to the "isolated alloc" fallback (a placeholder
`nil` move). The REAL first `field_store` (now three slots later than the
look-ahead expects) is then visited on its own in the next loop iteration,
where a dedicated guard rejects any `field_store` not immediately consumed by
an `alloc`'s look-ahead — producing exactly the `UnsupportedOp` error
`match`/`union` was failing with.

### 6.2 Fix chosen: reorder `twig-ir-compiler`'s codegen (option 2 from the
task), not `iir-to-beam`'s fusion look-ahead (option 1)

`box(field_name)` reads only the field value passed into the constructor —
it has no data dependency on `cell`, the fresh register `alloc` allocates.
Hoisting the `box` emission to BEFORE the `alloc` emission (source order)
produces exactly `[box, alloc, field_store, field_store]`, restoring the
`alloc`/`field_store`/`field_store` adjacency `iir-to-beam`'s look-ahead
requires, with **zero change to `iir-to-beam` or any other backend's
lowering**. This is a pure reorder of two independent SSA instructions: no
new register, no new op, no change to any instruction's operands — so it is
a no-op on every one of the other five backends (WASM/JVM/CLR/NativeAot/LLVM)
plus the two interpreter engines (Vm/Jit), all of which already ran this
exact constructor unchanged before this fix. It also exactly mirrors this
same function's OWN tag/head cons cell a few lines below, which already
computes its `box` before its `alloc` — so after this fix, both cons cells
`emit_union_def` builds follow one consistent "box first" convention, rather
than the per-field loop being the sole outlier.

This was chosen over teaching `iir-to-beam`'s fusion look-ahead to tolerate
(skip past) intervening non-`field_store` instructions — the alternative the
task explicitly offered. §6.1 shows the concrete case has exactly ONE
interleaved-instruction shape (a `box` with no register aliasing to the
`alloc`'d cell), not an open-ended set. A general "skip N non-conflicting
instructions while scanning for the fusion triple" scanner in `iir-to-beam`
would be strictly more code, in the shared backend every non-ALGOL frontend
depends on, to reach the exact same outcome a two-line reorder in the one
crate that produces the interleaving already achieves — over-engineering
this task's own instructions explicitly warn against ("don't over-engineer a
general 'skip N instructions' scanner if the concrete case only ever has one
specific interleaved instruction shape").

### 6.3 Proof

- `twig-ir-compiler`'s new
  `union_constructor_alloc_immediately_followed_by_its_two_field_stores`
  test (`tests/backend_compat.rs`) asserts, directly on the emitted IIR, that
  every `alloc ref<LispyPair>` instruction in the `Some` constructor is
  IMMEDIATELY followed by its car `field_store` (index 0) and then its cdr
  `field_store` (index 1) — the exact adjacency `iir-to-beam`'s fusion
  requires — for both cons cells the constructor builds (2 allocs checked).
  Confirmed this fails without the fix (rerun with the reorder temporarily
  reverted: the per-field cell's `alloc` is followed by `box`, not
  `field_store`, so the assertion fires with a clear message naming the
  interleaving instruction) before restoring the fix.
- `twig-ir-compiler`'s existing `union_constructor_boxes_tag_and_fields` test
  continues to pass unchanged — the reorder does not affect WHAT is boxed,
  only the instruction order, so the boxing invariant that test checks is
  untouched.
- `lang-aot`'s new `twig_beam_match_union` test (`tests/lang_matrix.rs`)
  compiles both promoted corpus rows through the FULL pipeline
  (`compile_source_to_beam`) and executes the resulting `.beam` bytes on
  real `erl`: `(match (Some 42) ((Some v) v) ((None) 0))` → 42, and
  `(match (None) ((Some v) v) ((None) 42))` → 42 (the second proving tag
  dispatch actually discriminates, not just the first arm). Both rows
  promoted to declare `Beam` in `lang_matrix.rs`'s `PROGRAMS` only after this
  test ran green against real `erl`, per this backlog's "probe before
  declaring, promote only proven cells" discipline.
- Full `iir-to-beam` suite (102 unit/integration + 23 lib + 5 doc tests) and
  full `twig-ir-compiler` suite (103 lib + 26 + 7 + 6 doc tests) re-run
  green after the fix — confirming zero regression on either crate's
  existing coverage. `cargo clippy -p twig-ir-compiler -p lang-aot
  --all-targets -- -D warnings` is clean.

### 6.4 Contract (what changed, this follow-up)

- `twig-ir-compiler/src/compiler.rs`: `emit_union_def`'s per-field loop now
  emits `box` before `alloc` (was: `alloc` then `box`). `iir-to-beam` is
  UNCHANGED by this follow-up.
- `twig-ir-compiler/tests/backend_compat.rs`:
  `union_constructor_alloc_immediately_followed_by_its_two_field_stores`.
- `lang-aot/tests/lang_matrix.rs`: both `match`/`union` Twig rows promoted to
  declare `Beam` (47/49 → **49/49**, Twig now complete on BEAM); new
  dedicated test `twig_beam_match_union`;
  `feature_coverage_doc_counts_match_programs_source`'s Twig tuple updated
  `(49, 390)` → `(49, 392)`.
- `code/specs/LANG-VM-FEATURE-COVERAGE.md`: Twig row (all 49/49 cells BEAM),
  grand-total prose (1663 → 1665 declared cells), and a new paragraph
  documenting this follow-up's pinned-down shape and fix.
- `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`: new top section (this
  follow-up) with the full investigation, reprioritizing the backlog now
  that Twig is fully closed.

### 6.5 What is now the only remaining non-ALGOL BEAM gap

With Twig at 49/49, COBOL-60 at 58/58, Nib/Oct/FLOW-MATIC/Brainfuck fully
proven per their own rows in `LANG-VM-FEATURE-COVERAGE.md`, the ONLY
remaining non-ALGOL BEAM gap in the whole backlog is Dartmouth BASIC's 8
still-undeclared rows: 5 `INPUT` rows (blocked on the unscoped BEAM
host-input design, VM-060b), 2 string-array/mixed-`DATA` rows (blocked on a
separate `str`-typed-array-element BEAM representation question — `iir-to-
beam` has none at all), and `RND` (blocked on VM-018's still-open
DEF-FN-and-module-global-chain design question).
