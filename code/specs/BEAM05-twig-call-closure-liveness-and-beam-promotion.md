# BEAM05 — Twig `call_closure` liveness fix and BEAM probe-first promotion

**Status:** Delivered — this slice (VM-041, first cut).
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

## 5. Explicitly out of scope / deferred

- `match`/`union` (2 Twig rows) — the `alloc`+`field_store` fusion-adjacency
  gap described in §3.2. Not attempted.
- Twig's remaining non-BEAM gaps outside this backlog's non-ALGOL scope are
  unaffected (this slice touches only `iir-to-beam`/`lang_matrix.rs`
  BEAM-column declarations).
- No full `non_algol_matrix_every_proven_cell_agrees` capstone rerun is
  claimed, matching every prior BEAM03/VM-LOOP-24/BEAM04 slice's own
  precedent — the two dedicated tests plus the full `iir-to-beam` suite
  (including the two new real-`erl` tests) are the executed evidence.
