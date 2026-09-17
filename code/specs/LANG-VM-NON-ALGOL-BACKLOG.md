# LANG VM non-ALGOL completion backlog

Status date: 2026-09-16

This is the execution backlog for completing the shared LANG VM platform while
the ALGOL campaign is owned separately. It complements
`LANG-FULL-IMPLEMENTATION.md`; when the two disagree about landed behavior,
executed tests and current package changelogs are authoritative until the older
roadmap is reconciled.

## BEAM08 — `RND` on BEAM: VM-018 closed, the non-ALGOL BEAM matrix is COMPLETE (selected by the user after BEAM07)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto the
BEAM07 merge (below, PR #15339). `gh pr list --state open --limit 50` showed
no other LANG-VM-related PR in flight, and specifically nothing touching
`iir-to-beam`, `dartmouth-basic-iir-compiler`, or `lang_matrix.rs`'s
`run_beam`. (A separate agent was reported concurrently working on T2
exceptions — a different runtime track, different files/crates — no overlap.)

BEAM07's own trailing note left exactly one item in this entire backlog:
`RND` (VM-018), the module-global design question every prior slice
(VM-LOOP-24, BEAM04, BEAM06, BEAM07) explicitly declined to force. The user
picked it directly: "RND sounds like a good one to fix up... please plan it
and execute on it."

Full research, decision, and validation: `code/specs/BEAM08-rnd-beam-support.md`.
Summary:

- **`RND` was never a genuinely new design question.** Direct inspection of
  `dartmouth-basic-iir-compiler`'s `emit_rnd_state_init`/`rnd_helper_function`
  shows `RND` shares its Park–Miller seed through the SAME `global_store`/
  `global_load` module-global substrate every other module-level BASIC/
  COBOL/Twig variable already uses — no bespoke RND-specific IIR op, no
  `DEF FN`-specific lowering. "RND's full DEF-FN-and-module-global chain"
  (VM-018's original framing) was, on inspection, just "a function that
  reads and writes a module global."
- **The trap (`{badarith,[{erlang,'*',[undefined,...]}]}`) matches a KNOWN,
  already-scoped bug exactly**: BEAM07's own VM-D036 discovery (issue
  [#15332](https://github.com/adhithyan15/coding-adventures/issues/15332))
  found `global_store` lowers `erlang:put/2` via `gc_bif2` — wrong, per real
  `erlc -S` disassembly, because `put/2` is not a guard-safe BIF and
  `gc_bif2`'s `Live` count cannot protect a co-live x-register across it.
  `main`'s initial `global_store` of the RND seed silently failed to stick
  under this bug, so the helper's first `global_load` read back `undefined`.
- **Confirmed concretely, not assumed**: converting `global_store` to
  `call_ext` (mirroring BEAM07's own fix for its new `put/2` usage) — with
  ZERO changes to `dartmouth-basic-iir-compiler`'s existing RND lowering —
  makes the exact promoted `RND` row produce the correct sequence on real
  `erl`. This confirms #15332 was both a NECESSARY and SUFFICIENT
  prerequisite, so it was fixed as part of this PR (a genuine dependency,
  not a drive-by) and closed, referencing this PR.
- **Fix**: `global_store`'s lowering converted from `gc_bif2` to `call_ext`
  (staging its value operand through a scratch register first — a
  parallel-move hazard, since the source register may already be x0 or x1
  — then `move key→x0`, `move value→x1`, `call_ext erlang:put/2`), protected
  by the existing `save_live_across_imported_call!`/
  `restore_live_across_imported_call!` Y-register machinery. Also added
  `"global_store"` to `iir-to-beam`'s `live_across` liveness filter, per
  issue #15332's own scope note — it was absent the whole time `gc_bif2`
  was assumed non-clobbering.
- **Re-verified the wide blast radius** issue #15332 flagged: `global_store`
  is used by every BASIC/COBOL/Twig program with a module-level variable.
  The full `iir-to-beam` suite (118 tests) and the full `lang_matrix.rs`
  BEAM test group (34 tests, spanning Twig/Nib/Oct/Brainfuck/COBOL-60/
  FlowMatic/Dartmouth BASIC) all pass unchanged — zero regressions. A full
  `non_algol_matrix_every_proven_cell_agrees` capstone rerun also passed
  clean (210 programs, 1467 cells exercised, 210 skipped for the
  host-wide missing `ilasm`, zero failures).

### Validation

`iir-to-beam` 0.16.0 → 0.17.0: `global_store`'s `call_ext` conversion +
`live_across` fix; 2 new tests —
`test_114_global_store_emits_call_ext_not_gc_bif2` (instruction-shape) and
`test_115_real_erl_global_store_survives_live_across_call` (the disposable
control test from issue #15332's own reproduction, made permanent: a
co-live `str_const` heap value now survives the call intact, and the
stored global reads back correctly). 118 tests total (up from 116);
`cargo clippy --all-targets -- -D warnings` clean.

`lang-aot` 0.345.0 → 0.346.0: the `RND` row promoted to declare `Beam` via
`portable_text_stdout_dartmouth_basic_beam_rnd`, executed against real
`erl` before promotion, per this backlog's "probe before declaring,
promote only proven cells" discipline — confirms stdout
`22\n85032\n85032\n601352`, matching every other standard backend exactly.
`feature_coverage_doc_counts_match_programs_source` updated (Dartmouth
BASIC tuple `(51, 407)` → `(51, 408)`); `LANG-VM-FEATURE-COVERAGE.md`'s
Dartmouth BASIC row, grand-total prose, and "Implemented feature families"
narrative row all updated to match.

**Dartmouth BASIC now declares all 51/51 rows on `Beam`.** Combined with
Twig (49/49), Nib (26/26), Oct (12/12), COBOL-60 (58/58), FLOW-MATIC (8/8),
and Brainfuck (3/6, intentional — real stdin-as-tape host support is a
separate, unscoped item unrelated to `RND`), **this closes every non-ALGOL
BEAM gap in this backlog.** The multi-month non-ALGOL BEAM completion track
this backlog has been driving since BEAM01 is now COMPLETE. Reprioritize
after this merges: the non-ALGOL BEAM matrix has no remaining probe-first,
design-question, or bounded-research items left; future non-ALGOL work in
this backlog (if any) would need a newly-scoped item (e.g. Brainfuck's
stdin-as-tape host support) rather than continuing this track.

## BEAM07 — BEAM host input: `INPUT`/`READ-ITEM` EOF peek, closing every non-ALGOL BEAM gap except `RND` (selected by the user after BEAM06)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto the
BEAM06 merge (below). `gh pr list --state open --limit 50` showed no other
LANG-VM-related PR in flight, and specifically nothing touching
`iir-to-beam`, `dartmouth-basic-iir-compiler`, `flow-matic-iir-compiler`, or
`lang_matrix.rs`'s `run_beam`.

BEAM06's own trailing note left exactly two items in this backlog: VM-060b
(BEAM host input, unblocking 5 BASIC `INPUT` rows + 4 FlowMatic
`READ-ITEM`/EOF rows at once) and VM-018 (`RND`'s module-global design).
The user picked VM-060b as the larger of the two.

Full research, decision, and the two pre-existing framework bugs found
along the way (both via genuine real-`erl` access-violation crashes, not
assumed): `code/specs/BEAM07-beam-host-input.md`. Summary:

- **Harness gap**: `run_beam` never piped a program's declared stdin to
  the spawned `erl` process at all — every other subprocess backend
  (native/LLVM/JVM/CLR) already did via `output_with_stdin`. Fixed: a pure
  extension (every other cell's `program_stdin` returns `b""`, a no-op).
- **IIR lowering gap**: `call_builtin "input_i64"`/`"input_str"`/
  `"input_more"` had no BEAM lowering at all (`UnsupportedOp` at
  validation). Fixed: `io:get_line/1` + `string:to_integer/1`/
  `string:trim/3` for the consuming reads (`input_i64`/`input_str`);
  `input_more`'s EOF peek emulated with a one-line lookahead cached in the
  process dictionary under a private key, consumed exactly once via
  `erlang:put/2`'s "returns the OLD value" contract — real `erl` has no
  byte-level peek primitive the way C's `ungetc` does.

### BEAM07 discovery: VM-D036 — `erlang:put/2` must lower to `call_ext`, not `gc_bif2` (found in this slice's own new code, but the identical bug is pre-existing in `global_store`)

Real `erlc` disassembly (`erlc -S`) confirms `erlang:get/1` compiles to a
zero-GC `bif` (safe via `gc_bif1`) but `erlang:put/2` — which can allocate,
growing the process dictionary's hash table — always compiles to a genuine
`call_ext`, never `gc_bif2`. This slice's own first cut of the `input_more`
lookahead cache used `gc_bif2` for `put/2` anyway, by direct precedent
from the EXISTING `global_store` lowering. It passed every single-read
test immediately, then either silently corrupted a separate live value or
crashed `erl` outright with a Windows access violation
(`ExitStatus(3221225477)` = `0xC0000005`) the moment two sequential reads
whose results both survive appeared in one function — exactly BASIC's
`INPUT A\nINPUT B\nPRINT A+B` and FlowMatic's `READ-ITEM` loop shape.

Bisected with a disposable control test using ZERO of this slice's new
code (`str_const` a list, an UNRELATED `global_store` of a plain integer,
read the list back) — it reproduced the identical access violation,
conclusive proof the bug is a **pre-existing `iir-to-beam` framework gap**,
not something this slice's new lowering introduced. `gc_bif2`'s `Live`
parameter cannot protect a separate live value across `erlang:put/2`,
because `put/2` does not honor a guard BIF's "only touch the declared
Dest" contract — no prior op combined "an `erlang:put/2` call" with "a
real heap value also live across it" until this slice's two-sequential-
reads shape did.

**Fixed in this slice's own new lowering** (`input_more`/`input_i64`/
`input_str`'s `put/2` calls now go through `call_ext`, protected by the
existing `save_live_across_imported_call!`/`restore_live_across_
imported_call!` Y-register machinery). **`global_store`'s identical bug is
NOT fixed here** — converting it requires also adding `global_store` to
`iir-to-beam`'s `live_across` liveness filter (currently absent, since
`gc_bif2` was assumed non-clobbering) and re-verifying the ENTIRE existing
BEAM corpus, since every BASIC/COBOL/Twig program with a module-level
variable goes through it. A correctly-scoped follow-up, not a drive-by fix
bundled into this feature slice — filed as
[#15332](https://github.com/adhithyan15/coding-adventures/issues/15332).

A second, related latent gap was found and fixed generally in the same
slice (not given its own VM-D0xx number, since it never manifested as an
independently-reproducible failure once VM-D036's fix landed — see the
spec's §3.3 for why both were verified significant regardless):
`lower_iir_to_beam`'s `{allocate, StackNeed, Live}` never zero-initializes
its Y-register slots the way a real `erlc` always does via `init_yregs` —
fixed with a `move {a,0} {y,N}` per slot immediately after `allocate`,
benefiting every function that spills to Y-registers, not just the new
host-input ops.

### Validation

`iir-to-beam` 0.15.0 → 0.16.0: new `call_builtin` lowering for the three
host-input builtins; the `erlang:put/2`-as-`call_ext` and `allocate`
Y-slot zero-init fixes; two new opcode constants (`OP_IS_INTEGER` = 45,
`OP_GET_TUPLE_ELEMENT` = 66, both confirmed via `beam_opcodes:opcode/2` on
real `erl`, OTP 29 erts-17.0.5, this host); 11 new tests (105 → 116) —
validation-acceptance, instruction-shape (each builtin calls exactly the
imports it should and none it shouldn't), and real-`erl` round-trips
(sequential reads summed/concatenated, EOF and parse-failure both return
the permissive default, `input_more` peek-then-consume across two records
plus a double-peek non-destructiveness proof). `cargo clippy --all-targets
-- -D warnings` clean.

`lang-aot` 0.344.0 → 0.345.0: `run_beam`'s stdin pipe; all 9 target rows
promoted to declare `Beam` — each executed against real `erl` with its
declared stdin piped through the fixed harness BEFORE being promoted, per
this backlog's "probe before declaring" discipline — via two new dedicated
tests, `portable_text_stdout_dartmouth_basic_beam_input` (5 rows) and
`portable_text_stdout_flow_matic_beam_read_item_and_eof` (4 rows).
`feature_coverage_doc_counts_match_programs_source` updated (Dartmouth
BASIC `(51, 402)` → `(51, 407)`; FLOW-MATIC `(8, 60)` → `(8, 64)`);
`LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth BASIC and FLOW-MATIC rows and
grand-total prose (1667 → 1676) updated to match. A full `lang_matrix` run
surfaced no regressions from this slice — the only other failures
(`matrix_every_proven_cell_agrees`, `proven_columns_do_not_silently_skip`,
and eight `algol_nested_procedure_*` tests) are the pre-existing,
separately-tracked ALGOL nested-procedure-captured-array bug on
NativeAOT/JVM (issue #12032), outside this task's ownership boundary and
unrelated to BEAM.

**Dartmouth BASIC now declares 50/51 rows on `Beam`** (only `RND`/VM-018
remains). **FLOW-MATIC now declares all 8/8 rows on `Beam`** — fully
complete. Combined with Twig (49/49), COBOL-60 (58/58), and Nib/Oct/
Brainfuck each fully proven per their own rows, **this closes every
non-ALGOL BEAM gap in this backlog except `RND`** (VM-018) and the
just-discovered, deliberately-deferred `global_store`/`gc_bif2` follow-up
(VM-D036) — both genuinely separate, unscoped questions a future slice
should pick up on their own merits, not probe-and-promote items.

## BEAM06 — Dartmouth BASIC BEAM: `str`-typed array element representation, 45/51 (selected after the VM-041 follow-up)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto the
VM-041 follow-up merge (below). `gh pr list --state open --limit 50` showed
no other LANG-VM-related PR in flight, and specifically nothing touching
`iir-to-beam`, `dartmouth-basic-iir-compiler`, or Dartmouth BASIC BEAM
support.

The VM-041 follow-up's own trailing note left the non-ALGOL BEAM backlog
down to EXACTLY three Dartmouth BASIC gaps, each framed as a genuine,
unscoped DESIGN question: 5 `INPUT` rows (VM-060b, out of scope per this
task's own instructions), `RND` (VM-018, out of scope), and 2 string-
array/mixed-`DATA` rows blocked on `iir-to-beam` having "no `str`-typed
array element representation at all." This slice picked the third — the
only one of the three that was not actually unscoped, once investigated —
and, per this task's own instructions, researched it via a dedicated spec
before implementing, mirroring BEAM04's own float-array-representation
precedent exactly.

### Research: does the existing `:ets` substrate already work for strings?

Rather than assuming new design work was needed (BEAM04 had framed this as
needing "a BEAM representation for a *heterogeneous* element type"), the
first step was checking whether BEAM04's existing `:ets`-backed array
substrate already worked for `str` elements unmodified — the same "does the
existing capability already cover this" check that found `neg`(f64) needed
zero new code in an earlier slice.

`str_const`'s existing scalar lowering (`iir-to-beam/src/lower.rs`) answered
the representation question directly: a v1 BEAM `str` value is already an
ordinary Erlang character list (`[byte, ...]`, built via `put_list`) — not
a handle, not a boxed reference. `:ets` stores arbitrary Erlang terms
natively (unlike `:atomics`, which is fixed-width 64-bit integers only), so
a character list is exactly as native to `:ets` as a float. Confirmed
directly on real `erl` (OTP 17.0.5, this host) with a standalone probe
mirroring `iir-to-beam`'s *exact* `array_set`/`array_get` instruction shape
byte-for-byte (`erlang:list_to_tuple([Idx, Val])` + `ets:insert/2` to
write, `ets:lookup_element/3` to read): round-trip, overwrite, concatenation
of two round-tripped elements, and the empty-string edge case (BEAM nil)
all passed — `ALL PROBES PASSED`.

Separately, BEAM04's "heterogeneous element" framing was traced directly
against `dartmouth-basic-iir-compiler`'s actual mixed-`DATA` lowering
(`emit_data_pool_init`/`emit_data_value`) and found not to apply: BASIC's
`DATA` pool materializes THREE separate parallel typed arrays (an
`array<i64>` kind-tag array, an `array<f64>` numeric-value array, an
`array<str>` string-value array) — never one array holding mixed element
types. Every `READ` checks the runtime kind tag, then reads from the
*statically*-chosen typed pool array. So no tagged/variant element
representation was needed anywhere in this backend, closing off the one
direction that might have made this a genuinely bigger item.

### Fix: widen the existing `:ets`-dispatch condition, nothing else

`iir-to-beam`'s `alloc_array`/`array_set`/`array_get` `:ets`-dispatch
conditions (`type_hint == "array<f64>"`/`"f64"`) each widened to also match
`"array<str>"`/`"str"`. The emitted BEAM instructions are byte-for-byte
identical to the existing float-array path — no branch on element type
inside the lowering itself. `validate.rs`'s Check 4 (`type_hint == "str"`
rejected unless the op is in a short allow-list) gained `"array_set"`/
`"array_get"`, mirroring the existing `str_const`/`str_concat`/`str_slice`/
`call`/`ret`/`mov` entries — `alloc_array`'s `"array<str>"` type_hint was
never rejected in the first place (Check 4 only matches the exact string
`"str"`). Zero new BEAM opcodes; `ir-to-beam` (the encoder) untouched.

Full research and decision write-up:
`code/specs/BEAM06-string-array-representation.md`.

### Validation

`iir-to-beam` 0.14.0 → 0.15.0: new unit test
`test_100_str_array_ops_use_ets_not_atomics` (instruction-shape: confirms
an all-`str`-array module's emitted `call_ext`s target `ets:*`/
`erlang:list_to_tuple/1` and none target `atomics:*`), plus two real-`erl`
tests — `test_101_real_erl_string_array_set_get_roundtrip` (the promoted
`DIM A$(2)` row's exact shape, prints `OK`) and
`test_102_real_erl_string_array_overwrite` (re-`array_set` at the same
index replaces, not duplicates, for `str` values exactly as
`test_96_real_erl_float_array_overwrite` already proved for `f64` values).
105 tests total (up from 102), all green; `cargo clippy -p iir-to-beam
--all-targets -- -D warnings` clean.

`lang-aot` 0.343.0 → 0.344.0: both Dartmouth BASIC rows VM-LOOP-24/BEAM04
left deferred on this exact gap — `DIM A$(2)` (plain string array) and the
mixed numeric/string `DATA`/`READ`/`RESTORE` row — promoted to declare
`Beam` via a new dedicated `portable_text_stdout_dartmouth_basic_beam_
string_arrays` test, each row executed against real `erl` before
promotion. `feature_coverage_doc_counts_match_programs_source` updated
(Dartmouth BASIC tuple `(51, 400)` → `(51, 402)`); `LANG-VM-FEATURE-
COVERAGE.md`'s Dartmouth BASIC row and grand-total prose (1665 → 1667)
updated to match. The broader `portable_text_stdout_` test group (36
tests), `t7_differential_random_basic_` group (4 tests), and
`feature_coverage_doc_counts_match_programs_source` all pass. A full
`non_algol_matrix_every_proven_cell_agrees` capstone rerun was attempted
but hit a pre-existing, unrelated failure (a Windows `erl` 17.0.5 access
violation, exit `-1073741819`, on an unrelated exponent-formatting row —
`10 PRINT 1.234567...`) — confirmed to reproduce byte-for-byte identically
on baseline `origin/main` code with this slice's changes stashed out, so it
is pre-existing local-host flakiness, not a regression from this slice.
This matches every prior BEAM03/VM-LOOP-24/BEAM04/VM-041 slice's own
precedent of not claiming a full capstone rerun as executed evidence for
this backlog.

**Dartmouth BASIC now declares 45/51 rows on `Beam`.** Only 6 rows remain
undeclared: the 5 `INPUT` rows (VM-060b) and `RND` (VM-018) — both
genuinely unscoped design questions, not probe-and-promote items. This
closes out every non-ALGOL BEAM gap in this backlog that does not require a
new, out-of-scope architectural decision. **Reprioritize after this
merges:** the non-ALGOL BEAM backlog has no remaining "probe before
declaring" or bounded-research items left — VM-060b (BEAM host input,
unblocking 5 BASIC + 4 FLOW-MATIC rows at once) and VM-018 (RND's module-
global design) are the only two items left, and both are genuinely
unscoped product/architecture questions a future slice should scope
properly before attempting, exactly as this task's own scope-boundary
section anticipated.

## VM-041 follow-up — Twig BEAM: `match`/`union` fusion gap pinned down and closed, Twig 49/49 (selected after VM-041 first cut)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto the
VM-041 first-cut merge (below, PR #15265). `gh pr list --state open --limit
50` showed no other LANG-VM-related PR in flight, and specifically nothing
touching `iir-to-beam`, `twig-ir-compiler`, or Twig BEAM support.

VM-041's own first cut left EXACTLY this problem open, described precisely:
Twig's last 2 undeclared `Beam` rows (`match`/`union`) failed lowering with
`UnsupportedOp { function: "Some", op: "field_store: found outside of
alloc+field_store+field_store pattern" }`, and the prior slice's own note
said the exact interleaving point was "not yet pinned down further, since
this slice deliberately did not force a fix here" — explicitly flagging it
as this backlog's next item.

### Pinning down the exact shape (required before any fix, per this task's
own instructions)

Compiled the `match`/`union` corpus program through
`lang_aot::compile_source_to_iir` and dumped the synthesized `Some`
constructor function's IIR directly (a scratch probe test, discarded before
this PR — not part of the permanent suite). The actual sequence:

```text
[0] const  _nil1                     : ref<LispyPair>
[1] alloc  _cell2                     : ref<LispyPair>   (field cons cell)
[2] box    _fbox3 = box(v)            : ref<any>          <-- interleaved
[3] field_store _cell2, 0, _fbox3     : void
[4] field_store _cell2, 1, _nil1      : void
[5] const  _tag4 = 0                  : i64
[6] box    _tbox5 = box(_tag4)        : ref<any>
[7] alloc  _head6                     : ref<LispyPair>   (tag/head cons cell)
[8] field_store _head6, 0, _tbox5     : void
[9] field_store _head6, 1, _cell2     : void
[10] ret   _head6                     : ref<LispyPair>
```

This settles both open questions precisely: the interleaved instruction is a
**`box`** (not the `mov` the prior slice's note had guessed — that guess
predates this direct inspection), and it sits between `alloc` and the FIRST
`field_store` (not between the two `field_store`s). The second cons cell this
same function builds (the tag/head cell, `[7]`–`[9]`) has NO interleaving —
its `box` already runs before its `alloc` in source order. Root cause:
`twig-ir-compiler::emit_union_def`'s per-field loop emits `alloc`, THEN
`box` (E6d-6b: boxing the field for the tagged backends' `match`/`unbox`
round-trip), THEN the two `field_store`s — `iir-to-beam`'s fusion look-ahead
peeks only at the NEXT TWO instructions after `alloc`, unconditionally, so
the interleaved `box` is misread as the first `field_store`, fails the op
check, and the fusion falls through to its "isolated alloc" fallback; the
real `field_store`s are then rejected on their own by a separate guard.

### Fix chosen: reorder `twig-ir-compiler`'s codegen, not `iir-to-beam`'s
fusion look-ahead

Of this backlog's own two named candidates — (a) teach the fusion
look-ahead to tolerate intervening non-`field_store` instructions, or (b)
change the union-variant constructor's codegen to avoid interleaving —
**(b) was chosen**. `box(field_name)` reads only the field value; it has no
data dependency on `cell`, the register `alloc` freshly allocates. Hoisting
`box` to before `alloc` produces `[box, alloc, field_store, field_store]` —
adjacency restored — with **zero change to `iir-to-beam`**: a pure reorder
of two independent SSA instructions is a no-op on every other backend
(WASM/JVM/CLR/NativeAot/LLVM/Vm/Jit all already ran this exact constructor
unchanged), and it exactly mirrors this same function's OWN tag/head cons
cell, which already boxes before allocating. Chosen over (a) because §6.1's
pinned-down shape shows exactly ONE interleaved-instruction case (a `box`
with no aliasing to the alloc'd cell) — a general N-instruction-skip scanner
in the shared BEAM backend every non-ALGOL frontend depends on would be
strictly more machinery, in the wrong crate, for the same outcome. This
follows this task's own explicit "don't over-engineer a general scanner"
guidance.

### Validation

`twig-ir-compiler` 0.45.0 → 0.45.1: `emit_union_def`'s per-field loop
reordered (`box` before `alloc`). New test
`union_constructor_alloc_immediately_followed_by_its_two_field_stores`
(`tests/backend_compat.rs`) asserts the adjacency directly on the emitted
IIR for both of the constructor's cons cells; confirmed to fail (naming the
interleaving `box`) with the fix temporarily reverted, then restored. The
existing `union_constructor_boxes_tag_and_fields` test (boxing invariant,
unaffected by instruction order) continues to pass. Full crate suite: 142
tests (103 lib + 26 + 7 backend_compat/backend_encode + 6 doc) pass.

`lang-aot` 0.342.0 → 0.343.0: both `match`/`union` Twig rows promoted to
declare `Beam` via a new dedicated test, `twig_beam_match_union`
(`tests/lang_matrix.rs`), each program executed against real `erl` before
promotion — `(match (Some 42) …)` = 42 and `(match (None) …)` = 42 (the
second proving tag-dispatch discrimination, not just the first arm).
`feature_coverage_doc_counts_match_programs_source` updated (Twig tuple
`(49, 390)` → `(49, 392)`) and passes against the live `PROGRAMS` corpus;
`LANG-VM-FEATURE-COVERAGE.md`'s Twig row and grand-total prose (1663 → 1665)
updated to match. `cargo clippy -p twig-ir-compiler -p lang-aot
--all-targets -- -D warnings` is clean. No full
`non_algol_matrix_every_proven_cell_agrees` capstone rerun is claimed for
this slice, matching every prior BEAM03/VM-LOOP-24/BEAM04/VM-041 slice's own
precedent — the dedicated tests plus the full `iir-to-beam`/`twig-ir-
compiler` suites are the executed evidence.

**Twig now declares 49/49 rows on `Beam` — fully complete.** This closes out
the last Twig BEAM gap. See `code/specs/BEAM05-twig-call-closure-liveness-
and-beam-promotion.md` §6 for the full investigation.

**Reprioritize after this merges — a genuine milestone.** With Twig at
49/49 and COBOL-60 already at 58/58 (and Nib/Oct/FLOW-MATIC/Brainfuck each
fully proven per their own rows), the non-ALGOL BEAM backlog is down to
EXACTLY ONE remaining gap: Dartmouth BASIC's 8 still-undeclared,
design-blocked rows — 5 `INPUT` rows (VM-060b's unscoped BEAM host-input
design), 2 string-array/mixed-`DATA` rows (a separate `str`-typed-array-
element BEAM representation question — `iir-to-beam` has none at all), and
`RND` (VM-018's still-open DEF-FN-and-module-global-chain design question).
Every one of those three remaining gaps is a genuine, unscoped DESIGN
question (not a probe-and-promote item like every VM-041/BEAM03/BEAM04/
VM-LOOP-24 item before it) — the next slice should pick ONE of the three and
write a dedicated design spec (mirroring BEAM04's own float-array-
representation research) before attempting any promotion, rather than
probing, since there is no more low-hanging "never actually run, not
actually broken" fruit left in this backlog.

## VM-041 — Twig BEAM: call_closure liveness fix + probe-first sweep (selected after BEAM04)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto the
BEAM04 merge (below). `gh pr list --state open --limit 50` showed no other
LANG-VM-related PR in flight, and specifically nothing touching
`iir-to-beam` or Twig BEAM support.

BEAM04's own trailing note named this exact item: Twig was 20/49 rows
declared `Beam`, with the remaining 29 framed only as unscoped "dynamic-
string/record/closure BEAM isolation" design work (VM-041) — no scoped
design existed yet. Per this task's own instructions, the confirmed
`call_closure` liveness bug (found by a prior agent and independently
verified by direct code read before this slice started) had to be fixed
FIRST, before any promotion — landing new closure-row Beam coverage on top
of a dormant silent-corruption bug would have been exactly backwards.

### The call_closure fix (VM-D035)

`iir-to-beam`'s `"call_closure"` lowering arm (`src/lower.rs`) emits TWO
`OP_CALL_EXT` instructions — `erlang:'++'/2` (combining captured values with
call arguments), then `erlang:apply/3` (invoking the closure) — and neither
was wrapped in the `save_live_across_imported_call!`/
`restore_live_across_imported_call!` macro pair every other call-emitting op
in this file uses, even though `call_closure` was already listed in the
`live_across` liveness match (the surrounding comment names the exact bug
class, VM-D029, already fixed once for the six `:atomics` ops). This is the
same "wrong VALUE, not a crash" failure mode: any SSA variable live across a
`call_closure` invocation could have its value silently destroyed if it
landed in an X register either call clobbers. Confirmed CURRENTLY DORMANT
before this fix: no `lang_matrix.rs` row exercising `call_closure` declared
`Beam` yet.

Fixed by wrapping BOTH `call_ext` emissions in a SINGLE
`save_live_across_imported_call!`/`restore_live_across_imported_call!` pair
spanning both calls — mirroring the `array_set` (f64/ets) arm's existing
pattern, which already wraps ITS two calls (`list_to_tuple` then
`ets:insert`) in one such pair. The call result is moved out of `x0` into
the destination register BEFORE the restore runs, mirroring `alloc_array`'s
ordering (restore can itself write back into `x0`).

Proven with a dedicated real-`erl` regression test,
`test_98_real_erl_call_closure_survives_live_across_call`: a variable `k`
defined before `call_closure` and used after it is allocated `x0` — the
exact register the closure dispatch's internal `caps -> x0` move clobbers
first — and the test asserts the correct combined result. Verified the test
actually exercises the bug (not merely a passthrough): reverting the fix
made the test fail with `14` instead of the correct `106` (the wrong value
`call_closure`'s own dispatch result leaked into `k`'s clobbered register),
confirmed by running the test with the fix temporarily removed before
restoring it.

### Probe-first sweep of the remaining 29 Twig rows

With the fix in place, a scratch probe (discarded before this PR, per this
task's own "probe before declaring" discipline) compiled and ran EVERY one
of the 29 not-yet-`Beam` Twig `lang_matrix.rs` rows individually against
real `erl`, using ONLY pre-existing `iir-to-beam` capabilities plus the fix
above.

**27 of 29 passed completely unchanged** — zero new lowering, zero new ops,
zero new imports:

- **8 dynamic-arithmetic/list-op/closure rows:** a bare literal, dynamic
  `any`-typed arithmetic over a `car`'d cons cell (`unbox`/`add`/`box`), the
  `length`/`list-ref`/`assoc` synthesized recursive list-walk helpers, and —
  thanks to the fix above — both closure rows (a no-capture closure and a
  capturing closure returning another closure).
- **19 string-op rows:** string literals, `let`/`let*` locals, non-escaping
  top-level `define`s, `substring`, `string<?`/`string>?` comparisons, the
  documented `string-ref` out-of-bounds trap, and a top-level function whose
  `str`-typed parameter is inferred four different ways (explicit
  annotation, a literal direct call, a `str_concat`+`str_slice` actual, and
  a named/`let`/`let*` actual). `iir-to-beam`'s string ops (`str_const`/
  `str_len`/`str_index`/`str_concat`/`str_slice`/`str_cmp`) were already
  proven on BEAM for other frontends (COBOL-60, Dartmouth BASIC) — these 19
  Twig rows simply had never been individually probed before, the same
  "never actually run, not actually broken" shape VM-LOOP-24 found for 12
  Dartmouth BASIC rows.

Promoted via two new dedicated `lang_matrix.rs` tests,
`twig_beam_dynamic_arith_list_ops_and_closures` (8 programs) and
`twig_beam_string_ops` (19 programs), each row executed against real `erl`
before promotion.

**2 of 29 (`match`/`union`) remain explicitly deferred, not forced.** While
probing, `iir-to-beam`'s validator was found to reject `"mov"` with a
`ref<LispyPair>` type_hint outright — but `lower.rs`'s `"mov"` arm already
lowers it correctly for ANY type_hint (an unconditional `{operand} ->
{x,rd}` move, agnostic to what the register holds). The gap traces to
`twig-ir-compiler::compiler.rs`'s `emit_move`, which merges each `if`/
`match` arm's result into one mutable "phi" variable via a typed `mov`
using the SOURCE value's own inferred type — `ref<LispyPair>` when a branch
is a cons cell (a `union` variant constructor's result). Relaxing the
validator (mirroring the existing `"str"`-type_hint exception already
there) is a real, tested fix — proven correct, not just accepted, by a new
dedicated real-`erl` test,
`test_99_real_erl_mov_ref_lispy_pair_lowers_correctly`, which merges two
cons cells built in the two arms of an `if` through the exact mutually-
exclusive two-`mov` pattern `emit_move` produces, and reads back the correct
one.

That fix alone is **not sufficient** to promote `match`/`union`: both rows
still fail with a SEPARATE, deeper error once validation passes —
`UnsupportedOp { function: "Some", op: "field_store: found outside of
alloc+field_store+field_store pattern — lower alloc+2×field_store into
put_list before reaching the backend" }`. `iir-to-beam`'s `alloc`+
`field_store`+`field_store` → `put_list` fusion (the pattern that lets a
single cons-cell construction lower to one BEAM `put_list`) only recognizes
the three instructions immediately ADJACENT in the instruction stream; the
synthesized union-variant constructor function (`Some`) interleaves a `mov`
between the `alloc` and its field_stores (or between the two field_stores —
not yet pinned down further), which breaks the look-ahead. This is a real,
scoped, still-open gap: the fusion look-ahead would need to tolerate (or
skip over) intervening non-`field_store` instructions, or the union-variant
constructor codegen would need to avoid interleaving them. Left deferred
with this exact description rather than guessed at or forced — genuinely
smaller and more precisely scoped than VM-041's original "dynamic-string/
record/closure BEAM isolation" framing, since strings, dynamic arithmetic,
list ops, and closures are now ALL proven; only tagged-union pattern
matching remains.

### Validation

`iir-to-beam` 0.13.0 → 0.14.0: 102 unit/integration tests + 5 doc tests pass
(up from 100 + 5), including the two new tests above (`test_98` proves the
liveness fix; `test_99` proves the validator fix lowers correctly, not just
validates). All-target Clippy with warnings denied is clean.

`lang-aot` 0.341.0 → 0.342.0: promoted 27 Twig `lang_matrix` rows to `Beam`
(20 → 47 of 49) via the two new dedicated tests above, each program executed
against real `erl` before promotion.
`feature_coverage_doc_counts_match_programs_source` was updated (Twig tuple
`(49, 363)` → `(49, 390)`) and passes against the live `PROGRAMS` corpus;
`LANG-VM-FEATURE-COVERAGE.md`'s Twig row, grand-total prose (1636 → 1663),
and the VM-041 narrative paragraph were all updated to match. No full
`non_algol_matrix_every_proven_cell_agrees` capstone rerun is claimed for
this slice, matching every prior BEAM03/VM-LOOP-24/BEAM04 slice's own
precedent — the two dedicated tests above plus the full `iir-to-beam` suite
(including the new real-`erl` tests) are the executed evidence.

Twig now declares 47/49 rows on `Beam` — up from 20/49. Only 2 rows remain
undeclared: `match`/`union`, gated on the `alloc`+`field_store` fusion gap
described above — a genuinely bounded follow-up now that every other Twig
feature family (dynamic arithmetic, list ops, strings, closures) is proven
on BEAM. Twig no longer has a large undeclared surface; VM-041 is
effectively DONE except for this one precisely-scoped fusion gap.
Reprioritize after this merges: the only two remaining non-ALGOL BEAM gaps
across the whole backlog are (a) this `match`/`union` fusion gap (2 Twig
rows) and (b) Dartmouth BASIC's remaining 8 rows (5 `INPUT` rows on VM-060b,
2 string-array/mixed-`DATA` rows on a separate `str`-typed-array-element
design question, and `RND` on VM-018's module-global design question) — the
non-ALGOL BEAM backlog is close to fully closed.

## BEAM04 — ets-backed float-array representation (selected after VM-LOOP-24)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto the
VM-LOOP-24 merge (below). `gh pr list --state open --limit 50` showed no
other LANG-VM-related PR in flight, and specifically nothing touching
`iir-to-beam` or BASIC array/float support.

VM-LOOP-24's own Findings §1 (and its trailing reprioritization note) named
this exact problem: `iir-to-beam` represents every `alloc_array`/
`array_set`/`array_get` with Erlang's `:atomics` module, which can only hold
64-bit INTEGERS — confirmed by a real trap
(`{badarg,[{atomics,put,[Ref,Index,FloatValue],...}]}`) the instant a
float value is written. BASIC arrays are `array<f64>` (every scalar numeric
value routes through the shared f64 track per BA7-1b), so every BASIC array
write traps, blocking 4 numeric-array/`DATA` rows.

**This is a design item, not a promotion, so it needed real research before
any code.** Wrote `code/specs/BEAM04-float-array-representation.md`,
researching both candidates the backlog named — concretely, against real
`erl`/`erlc -S`, not by intuition:

- **Bit-reinterpret the f64 as an i64, keep `:atomics`.** The bit pattern
  DOES round-trip exactly (verified across a wide set of finite values,
  including `±0.0`, min/max-magnitude doubles, and ordinary decimals) — but
  the actual BEAM instruction shape needed, confirmed by disassembling a
  real compiled bit-syntax round-trip with `erlc -S`, requires THREE
  entirely new opcode families this backend has never implemented:
  `bs_create_bin` (construction), `bs_start_match4`/`bs_match` (matching,
  with a nested `{commands, [...]}` operand shape unlike anything this
  encoder currently handles), and a typed `test bs_get_float2` extraction.
- **Switch float-element arrays to `:ets`.** Disassembly confirmed this
  needs **zero** new BEAM opcodes: `ets:new/2`/`ets:insert/2`/
  `ets:lookup_element/3` are ordinary `call_ext`s (the exact shape
  `math:*` already uses), and the `{Idx, Val}` insert tuple is built with
  `put_list` (the exact cons-cell construction `call_closure`'s arg-list
  already uses) + `erlang:list_to_tuple/1`. Functional testing on real
  `erl` (not just disassembly) confirmed: no size argument needed (`:ets`
  grows dynamically, unlike fixed-size `:atomics`), a fixed table-name atom
  never collides across allocation sites (`ets:new` without `named_table`
  always returns a fresh identifier), re-`insert` at an existing key
  overwrites (matching `atomics:put`'s semantics), and a missing-key read
  traps `badarg` (matching `atomics:get`'s out-of-range failure mode).

**`:ets` was chosen.** This backlog's own prior framing guessed that reusing
`:atomics` "is likely simpler... since it wouldn't need a new
table-lifecycle model" — that guess did not survive measurement. `:ets`
needs zero new opcodes (pure reuse of `put_list`/`call_ext`, both already
implemented and tested); the bit-reinterpretation path needs a materially
larger, structurally new part of the encoder for a benefit (staying on
`:atomics`) that turned out not to matter — no per-array table-lifecycle
problem ever materialized (no explicit cleanup needed; a single `call_ext`
creates a table exactly like `atomics:new` does).

### Findings

- One genuine, documented limitation was found and deliberately left
  unfixed: unlike `atomics:new`, `ets:new` does not pre-zero N cells, so
  `array_get` on an index that was never `array_set` traps `badarg` instead
  of returning `0.0`. Confirmed not to affect any of the 4 rows being
  promoted (each writes every cell it later reads — checked against each
  row's actual source before promoting, not assumed) and pinned by a
  dedicated real-`erl` test (`test_97_real_erl_float_array_unset_read_traps`)
  so it can't regress silently into "returns garbage" later.
- No other frontend or existing corpus row was put at risk: only
  `array_set`/`array_get`/`alloc_array` instructions whose `type_hint` is
  exactly `"f64"`/`"array<f64>"` dispatch to `:ets`; every other array/tape
  use (Brainfuck's byte tape, the GOSUB return-address `array<i64>` stack,
  the `DATA` pool's kind array) is untouched, and — since EVERY f64 array
  op traps on real `erl` today, per VM-LOOP-24's own finding — there was no
  working case to regress.

### Validation

`iir-to-beam` 0.12.0 → 0.13.0: 104 unit/integration tests + 5 doc tests pass
(up from 100), including a new instruction-shape test
(`test_94_f64_array_ops_use_ets_not_atomics`) that checks the actual
`call_ext` OPERANDS emitted target `:ets`/`list_to_tuple` and none target
`:atomics` (import-table presence alone can't prove this — every import is
pre-registered unconditionally at module setup), and three new real-`erl`
integration tests: a set/get roundtrip matching the promoted 1-D-array row
exactly (`test_95`), an overwrite proof (`test_96`), and the
unset-read-traps proof above (`test_97`). All-target Clippy with warnings
denied is clean.

`lang-aot` 0.340.0 → 0.341.0: promoted the 4 Dartmouth BASIC `lang_matrix`
rows VM-LOOP-24 found blocked on this gap — 1-D array, 2-D array, `DATA`/
`READ`/`RESTORE`, and fractional `DATA` — via a new dedicated
`portable_text_stdout_dartmouth_basic_beam_arrays_and_data` test (4
programs) executed against real `erl` before promotion.
`feature_coverage_doc_counts_match_programs_source` was updated
(Dartmouth BASIC tuple `(51, 396)` → `(51, 400)`) and passes against the
live `PROGRAMS` corpus; `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth BASIC row,
grand-total prose (1632 → 1636), and the "Implemented feature families"
narrative row were all updated to match. No full
`non_algol_matrix_every_proven_cell_agrees` capstone rerun is claimed for
this slice, matching every prior BEAM03/VM-LOOP-24 slice's own precedent —
the dedicated arrays/DATA test plus the full `iir-to-beam` suite (including
the new real-`erl` tests) are the executed evidence.

Dartmouth BASIC now declares 43/51 rows on `Beam` — up from 39/51. Only 8
rows remain undeclared: the 5 `INPUT` rows (still gated on VM-060b), the 2
string-array/mixed-`DATA` rows (gated on a SEPARATE, larger design item —
`iir-to-beam` has no `str`-typed array element representation at all,
independent of the float-storage question this slice closed), and `RND`
(gated on the VM-018 module-global design question). Reprioritize after
this merges: **VM-060b (BEAM host input) is now the single highest-value
remaining non-ALGOL BEAM item for Dartmouth BASIC**, unblocking 5 rows at
once — the largest remaining single group for this frontend. The
string-array representation gap is the next-largest design item (only 2
rows, but architecturally bigger than the float-storage question just
closed, per §5 of the BEAM04 spec). `RND` is a single row gated on a design
question unrelated to arrays or input (VM-018), not attempted here or by
any prior slice. VM-041 (Twig dynamic-string/record/closure BEAM isolation)
remains the other standing non-ALGOL candidate untouched by this slice.

## VM-LOOP-24 — probe-first sweep of the remaining Dartmouth BASIC non-`INPUT` rows (selected after #15231)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto
`58fbc2f1a5` (PR #15231, the BEAM03 continuation `neg`(f64)/`f64_pow` slice).
`gh pr list --state open --limit 50` showed no other LANG-VM-related PR in
flight, and specifically nothing touching `iir-to-beam` or BASIC BEAM support.

That merged slice's own trailing note named the next work precisely: Dartmouth
BASIC was 22/51 rows on `Beam`, and the remaining ~29 rows split into (a) 5
`INPUT` rows (BASIC) + 4 `INPUT` rows (FLOW-MATIC) needing the unscoped
VM-060b BEAM host-input design — explicitly out of scope, not attempted here
— and (b) the rest (`FOR`/`LET` general arithmetic, one- and two-D numeric
arrays, `DATA`/`READ`/`RESTORE`, `RND`'s full DEF-FN-and-module-global chain,
`GOSUB`/`RETURN`), each needing its OWN real-`erl` probe before promotion,
explicitly flagging that "nothing in this slice or its predecessor
established that neg/f64_pow alone unblocks them".

**This is a probe-first task, exactly as scoped.** A scratch probe (discarded
before this PR — not part of the permanent test suite) compiled and ran EVERY
one of the 24 not-yet-`Beam`, non-`INPUT` Dartmouth BASIC corpus rows
individually against real `erl`, unmodified from their existing
`lang_matrix.rs` source text, using only the `iir-to-beam` capabilities that
existed before this slice.

### Findings

- **12 rows ran with ZERO new lowering.** General `FOR`/`FOR … STEP` loops,
  `IF … THEN` + `GOTO`-style line jumps, a same-module `DEF FN` call,
  multi-item `PRINT` with `;` (including a literal negative, i.e. `sub`) and
  `,`, ordinary scalar real arithmetic, BA7 fixed-decimal and six-
  significant-digit/`E`-notation formatting, flat and nested `GOSUB`/
  `RETURN`, and `SGN`. None of these needed anything BEAM03/BEAM03-
  continuation hadn't already proven — they simply hadn't been run as
  individual corpus rows before. `GOSUB`/`RETURN` in particular reuses the
  E5 `array<i64>` return-address stack, which is an INTEGER array, so it
  never touches the float-array gap found below.
- **5 more rows needed new but bounded/mechanical lowering.**
  `SQR`/`SIN`/`COS`/`LOG`/`EXP`/`ATN`/`TAN` each lower to a single-source
  IIR op (`f64_sqrt`/`f64_sin`/`f64_cos`/`f64_ln`/`f64_exp`/`f64_atan`/
  `f64_tan`) that a real `erlc -S` disassembly confirmed emits `call_ext`
  against a `math:*` import (never `gc_bif1` — `math` functions are
  ordinary library code, not loader-recognized guard BIFs) — the exact
  `f64_pow` `call_ext` shape from the prior slice, just with one operand
  staged instead of two. `INT` additionally needed `real_to_int_floor`,
  confirmed (also via `erlc -S`) to disassemble to the SAME generic
  `gc_bif1` shape `int_to_real`/`real_to_int_trunc` already use
  (`erlang:floor/1` — a genuine guard BIF, unlike the `math:*` functions
  above it). Implemented both additions in `iir-to-beam` 0.11.0 → 0.12.0,
  mirroring the existing patterns exactly (see
  `code/specs/BEAM03-float-lowering.md` §9.3 for the full derivation).
- **3 groups are explicitly deferred as genuine design questions, per this
  task's own "do not force a design decision" instruction:**
  1. **1-D/2-D numeric arrays and `DATA`/`READ`/`RESTORE`** (4 rows) compile
     cleanly through `iir-to-beam` validation, then TRAP AT RUNTIME on real
     `erl` with `{badarg,[{atomics,put,[Ref,Index,FloatValue],...}]}`.
     `iir-to-beam` represents every `alloc_array`/`array_set`/`array_get`
     with Erlang's `atomics` module, and `atomics` is a fixed-size array of
     **64-bit INTEGERS ONLY** — confirmed directly on real `erl`
     (`atomics:put(Ref, 1, 40.0)` traps; the integer form does not). BASIC
     arrays are `array<f64>` (even `DIM A(3)`, since BA7-1b routes every
     scalar numeric value through the shared `f64` track), so every BASIC
     array write traps. Fixing this needs a real architecture decision (bit-
     reinterpreting the float as an integer via BEAM bit-syntax — itself new
     lowering `iir-to-beam` has for no op today — versus a different mutable
     substrate entirely, e.g. `ets`), not a mechanical opcode addition. Left
     open.
  2. **String arrays and mixed numeric/string `DATA`** (2 rows) fail
     `iir-to-beam` VALIDATION outright (`UnsupportedType`: `str`-typed
     `array_set`/`array_get`), independent of the float-array gap above —
     `iir-to-beam` has no `str`-typed array element representation at all.
     A separate, likely larger design item. Left open.
  3. **`RND`** traps with `{badarith,[{erlang,'*',[undefined,48271],...}]}`
     inside its compiled `__basic_rnd` helper — a module-global
     `erlang:get/1` read returns `undefined` instead of the expected seed.
     This is the SAME "RND's full DEF-FN-and-module-global chain" open
     design question flagged at VM-018, confirmed still unresolved and
     confirmed NOT fixed as a side effect of this slice's `real_to_int_floor`
     addition (the probe re-ran `RND` after implementing it). Left open.

### Validation

`iir-to-beam` 0.12.0: 96 unit/integration tests + 5 doc tests pass (up from
92), including new tests proving each of the 7 `math:*` transcendentals
emits exactly one `call_ext` (never `gc_bif1`/`gc_bif2`) against the correct
import (`test_90`), `real_to_int_floor` emits `gc_bif1` against
`erlang:floor/1` (not `call_ext`, unlike its `math:*` siblings — `test_91`),
and two new real-`erl` integration tests: all 7 transcendentals chained in
one module against their exact Dartmouth BASIC corpus arguments (`SQR(49) +
SIN(0) + COS(0) + LOG(1) + EXP(0) + ATN(0) + TAN(0)` truncated = `9`,
`test_92`) and `real_to_int_floor(3.7)` = `3` (`test_93`). All-target
Clippy with warnings denied is clean.

`lang-aot` 0.339.0 → 0.340.0: promoted 17 more Dartmouth BASIC `lang_matrix`
rows to `Beam` (22 → 39 of 51) via two new dedicated tests executed against
real `erl` before promotion —
`portable_text_stdout_dartmouth_basic_beam_general_arithmetic_and_control_flow`
(12 programs, zero new lowering) and
`portable_text_stdout_dartmouth_basic_beam_math_builtins` (5 programs, the
new `math:*`/`real_to_int_floor` lowering). `feature_coverage_doc_counts_match_programs_source`
was updated (Dartmouth BASIC tuple `(51, 379)` → `(51, 396)`) and passes
against the live `PROGRAMS` corpus; `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth
BASIC row, grand-total prose (1615 → 1632), and the "Implemented feature
families" narrative row were all updated to match. No full
`non_algol_matrix_every_proven_cell_agrees` capstone rerun is claimed for
this slice, matching both BEAM03 slices' own precedent — the two dedicated
tests above plus the full `iir-to-beam` suite (including the new real-`erl`
tests) are the executed evidence.

Dartmouth BASIC now declares 39/51 rows on `Beam` — up from 22/51, and up
from 0/51 three slices ago. Only 12 rows remain undeclared: the 5 `INPUT`
rows (still gated on VM-060b), the 6 array/`DATA` rows (gated on a BEAM
float/string array data-representation decision — see Findings above), and
`RND` (gated on the VM-018 module-global design question). Reprioritize
after this merges: **VM-060b (BEAM host input) and the BEAM float-array
representation decision are now the two highest-value non-ALGOL BEAM items**
— either would unblock the largest remaining single group (5 and 4 rows
respectively) — but both are genuine design items, not quick probes; picking
between them (or working the array-representation research as its own
bounded design spike, the way BEAM03's own `LitT`/zlib research was) is the
next real product decision for this backlog, not something to default into
without deciding. VM-041 (Twig dynamic-string/record/closure BEAM isolation)
remains the other standing non-ALGOL candidate untouched by this slice.

## BEAM03 continuation — neg(f64) and f64_pow (selected after #15219)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto
`f5f657a3f9` (PR #15219, the first BEAM03 f64-lowering slice). `gh pr list
--state open --limit 50` showed no other LANG-VM-related PR in flight, and
specifically nothing touching `iir-to-beam` or BASIC float support.

That merged slice's own trailing note named the next two candidates
explicitly: (a) `neg`(f64) — "structurally trivial (mirrors `neg`(i64)'s
existing `gc_bif1` shape with `erlang:'-'/1`, already polymorphic over int
and float)"; (b) `f64_pow` — "no direct single-BIF equivalent (`math:pow/2`
returns a float already, so likely a straightforward `gc_bif2`-shaped
`call_ext`, but unverified)". This slice implements both, in one PR (the
task's own scope note allowed splitting into two if `f64_pow` turned out
non-trivial, but it stayed simple enough — a `call_ext`, just not a
`gc_bif2` — to land together).

### Findings

- **`neg`(f64) needed zero lowering changes.** Reading `lower.rs`'s existing
  `"neg" | "not"` arm confirmed the prediction exactly: it dispatches
  unconditionally to `import_neg` (`erlang:-/1`), and the only
  `type_hint`-conditional branch in that arm is the `u4`/`u8` narrowing
  mask, which `"f64"` never matches. `validate.rs` never special-cased `neg`
  either. Real `erl` confirms `erlang:'-'(3.14)` returns `-3.14` — no
  `badarith`, unlike `div` (VM-D034's two-different-operators case). This is
  a genuinely different shape from VM-D034: there, integer and float needed
  DIFFERENT Erlang operators; here, one operator already covers both. New
  unit + real-`erl` tests exist anyway, because "reading the code says it's
  already right" and "proving it's already right" are not the same
  discipline, and this backlog treats them as such.
- **`f64_pow` needed the `call_ext` path, not `gc_bif2`.** The "unverified"
  flag on this candidate was warranted: `math:pow/2` is an ordinary Erlang
  function, not a loader-recognized guard BIF (confirmed by reading
  `beam_asm.erl`'s guard-BIF table and by disassembling a real compiled
  `math:pow/2` call with `erlc -S`, which showed `call_ext`, never
  `gc_bif2`). It lowers the same way `str_concat`'s `erlang:'++'/2` and
  `str_index`'s `lists:nth/2` already do: stage both operands into scratch
  registers above `next_reg`, save live-across-call variables, move into
  `x0`/`x1`, `call_ext 2 {u,import_pow}`, move the result, restore live
  variables. `f64_pow` was added to the `live_across` call-list match — the
  same "EVERY op that emits a `call_ext` must be listed here" invariant
  documented at that match (an omission there silently destroys a live
  variable's value, not a crash — the exact bug class a prior slice already
  found and fixed for the six `:atomics` ops and `call_closure`).
- No new platform-limitation surface: `f64_pow`'s Inf/NaN behavior is
  exactly the same already-documented (BEAM03-float-lowering.md §6)
  Erlang-floats-cannot-represent-Inf/NaN gap `div` has — real Erlang's
  `math:pow/2` is a direct libm `pow` binding, matching the `vm-core`
  oracle's `f64::powf` semantics on every finite input, diverging only where
  every backend's float division already diverges. Not reachable by the
  promoted row (base `4`, exponent `0.5`, exact finite result `2.0`).

### Validation

`iir-to-beam` 0.11.0: 92 unit/integration tests + 5 doc tests pass (up from
87), including new tests proving `neg`(f64) emits exactly one `gc_bif1` and
zero masking `gc_bif2`s, `f64_pow` emits exactly one `call_ext` (not
`gc_bif2`) against a real `math:pow/2` import-table entry, and three new
real-`erl` integration tests: double negation (`neg(neg(3.5))` = `3.5`),
`f64_pow(2.0, 10.0)` truncated = `1024`, and a combined case chaining `neg`
+ `f64_pow` in one module reproducing the `4 ^ 0.5` BASIC row's exact
arithmetic. All-target Clippy with warnings denied is clean.

`lang-aot` 0.339.0: promoted two more Dartmouth BASIC `lang_matrix` rows to
`Beam` — `10 PRINT ABS(-42)` (exercises `neg`(f64) twice: once for the
literal `-42` via `emit_unary`, once inside ABS's own inline conditional
negation) and `10 PRINT 4 ^ 0.5` (a fractional exponent that misses the
literal-integer-exponent fast path and falls through to the general
`f64_pow` runtime call). Both executed on real Erlang via a new dedicated
`portable_text_stdout_dartmouth_basic_beam_neg_and_pow` test before
promotion, matched by exact source text (not position — these two rows are
not adjacent to each other or to the BEAM03 numeric-baseline pair; SGN/ATN/
TAN sit between ABS and the `^` row). `feature_coverage_doc_counts_match_programs_source`
was updated (Dartmouth BASIC tuple `(51, 377)` → `(51, 379)`) and passes
against the live `PROGRAMS` corpus; `LANG-VM-FEATURE-COVERAGE.md`'s
Dartmouth BASIC row, grand-total prose (1613 → 1615), and the "Implemented
feature families" narrative row were all updated to match. No full
`non_algol_matrix_every_proven_cell_agrees` capstone rerun is claimed for
this slice, matching the first BEAM03 slice's own precedent — the dedicated
neg/pow test plus the full `iir-to-beam` suite (including the new real-`erl`
tests) are the executed evidence.

Dartmouth BASIC now declares 22/51 rows on `Beam` (up from 20/51).
Reprioritize after this merges: the remaining ~29 numeric/`INPUT` BASIC rows
split into two independent gaps — the 5 `INPUT` rows need the unscoped BEAM
host-input design (VM-060b, shared with FLOW-MATIC's 4 `INPUT` rows); the
rest (`FOR`/`LET` general arithmetic, one- and two-D numeric arrays, `DATA`/
`READ`/`RESTORE`, `RND`'s full DEF-FN-and-module-global chain, `GOSUB`/
`RETURN`) each need their OWN real-`erl` probe before promotion — nothing in
this slice or its predecessor established that `neg`/`f64_pow` alone
unblocks them, and several (the array ops, the `GOSUB` return-address stack)
already ran on BEAM via unrelated ops in other frontends, so they may
already be one probe away rather than needing new lowering work. The
still-open float-division-by-zero design question (§6) also remains
unraised — no promoted row anywhere yet exercises a runtime-variable
divisor. VM-041 (Twig dynamic-string/record/closure BEAM isolation) remains
the other standing non-ALGOL candidate untouched by either BEAM03 slice.

## BEAM03 — iir-to-beam f64 lowering, first bounded slice (selected after #15147)

`git fetch origin && git merge origin/main` fast-forwarded cleanly onto
`bebf50d125` (PR #15147, the Dartmouth BASIC BEAM pure-string family).
`gh pr list --state open --limit 50` showed no LANG-VM-related PR in flight,
and specifically nothing touching `iir-to-beam` or BASIC float support.

That merged slice's own trailing note was unambiguous about the next item:
`iir-to-beam` has **zero** `f64`/`Float` lowering support at all (no
`f64`/`Float`/`fadd`-family match arm anywhere in `lower.rs`), and BASIC's
BA7-1b change routes every scalar numeric value — even an integer-spelled
literal like `PRINT 42` — through the shared `f64` value track, blocking
~28 of BASIC's 51 corpus rows. The note explicitly called this out as
qualitatively different from every prior VM-040 BEAM slice (which only ever
needed to *declare* an already-working op) and recommended scoping it as its
own real production-code track, likely multi-slice, the way COBOL BEAM was.

**This is a design item, not a promotion, so it needed real research before
any code.** Wrote `code/specs/BEAM03-float-lowering.md` covering:

- The exact `LitT` (literal table) chunk binary layout, reverse-engineered
  from a real compiled Erlang module's chunks (`beam_lib`/`beam_disasm`) and
  cross-checked against `beam_asm.erl` (OTP's own assembler source, shipped
  locally at `lib/compiler-10.0.3/src/beam_asm.erl`) rather than assumed
  from memory — the "read the actual runtime contract" discipline the VM-D029
  signed-integer fix established.
- A previously-unnoticed OTP-version split: OTP 28+ stores `LitT`
  **uncompressed**, but this repo's CI pins **OTP 27.3.4.11**
  (`.github/workflows/ci.yml`), which only accepts the older
  **zlib-compressed** form. Implemented a dependency-free RFC 1950/1951
  encoder (stored/uncompressed DEFLATE blocks only — a small, unambiguous
  special case any compliant decoder must accept) rather than add this
  repo's first external compression crate; every other binary encoder in
  `ir-to-beam`/`iir-to-beam` is already hand-rolled with zero dependencies.
- A genuine platform limitation, not a lowering gap: real Erlang floats
  cannot represent IEEE-754 Inf/NaN at all (`X/0.0` raises `badarith`, and
  the ETF decoder itself refuses a non-finite float bit pattern). This
  diverges from the `vm-core` oracle's IEEE-754 contract (matched by every
  other backend's `fdiv`) and has no clean fix — left as an explicit open
  design question for a future slice, not guessed at.

Every claim above was verified against a real, locally-installed Erlang/OTP
29 runtime before being relied on — including patching a real compiled
`.beam` file's `LitT` chunk with the exact byte layout this slice implements
and successfully loading and running it with real `erl`, per this loop's own
"probe before declaring" discipline, applied here to the *platform* rather
than to a frontend's corpus row.

### BEAM03 contract

Implement, in `ir-to-beam` (encoder): the `Z` extended compact-term tag,
`BEAMModule.literals`, `literal_operand`, `etf_new_float`, the dependency-free
zlib stored-block encoder, and `LitT` chunk assembly (omitted when empty).
In `iir-to-beam` (lowering): accept `const` with `Operand::Float` when
`type_hint == "f64"` (previously rejected unconditionally); add
`int_to_real`/`real_to_int_trunc` via new single-argument `gc_bif1` BIF
calls (`erlang:float/1`/`erlang:trunc/1`); leave `add`/`sub`/`mul`/`cmp_*`
untouched (they already lower generically over any register contents).
Promote the two Dartmouth BASIC "numeric baseline" rows (`PRINT 42` and
`PRINT 6 ^ 2 + 6`) to `Beam` only after executing them on real Erlang.
Explicitly out of scope: the remaining ~26 BASIC numeric/`INPUT` rows
(need `neg`(f64), `f64_pow`, and VM-060b host input), `f32`, and general
float division by zero (the open platform-limitation question above).

### BEAM03 validation

`ir-to-beam` 0.4.0: 77 unit tests + 5 doc tests pass, including new
Adler-32 known-vector, stored-block round-trip (small, empty, and a
>64KiB multi-block case), `literal_operand` byte-level encoding, and
`LitT`-chunk-presence/omission tests. All-target Clippy with warnings
denied is clean.

`iir-to-beam` 0.10.0: 87 unit/integration tests + 5 doc tests pass,
including new real-`erl` integration tests for float arithmetic
(`((3.5+2.5)-1.0)*2.0/4.0` truncated → `2`), float comparisons
(`cmp_lt`/`cmp_eq`/`cmp_ge` summed → `3`), and `int_to_real`/
`real_to_int_trunc` round-trip (`trunc(float(7)+0.5)` → `7`) — all executed
against real Erlang, not just validated for instruction shape. Writing the
float-division test surfaced **VM-D034**: the existing i64 `div` lowering
used `erlang:div/2`, which traps (`badarith`) on a float operand (confirmed:
`erlang:div(10.0, 4.0)` traps on real `erl`); fixed by dispatching `div` to
a new `erlang:'/'/2` import specifically when `type_hint == "f64"`, leaving
the i64 path unchanged. All-target Clippy with warnings denied is clean.

`lang-aot` 0.338.0: both promoted BASIC rows pass on real Erlang via the new
dedicated `portable_text_stdout_dartmouth_basic_beam_numeric_baseline` test
(mirroring the pure-string family's own test shape), and all 23 BEAM-prefixed
`lang_matrix` tests (22 prior + this new one) pass together.
`feature_coverage_doc_counts_match_programs_source` was updated (Dartmouth
BASIC tuple `(51, 375)` → `(51, 377)`) and passes against the live `PROGRAMS`
corpus; `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth BASIC row and grand-total
prose were updated to match (1611 → 1613 declared cells). No full
`non_algol_matrix_every_proven_cell_agrees` capstone rerun is claimed for
this slice — the dedicated numeric-baseline test plus the full `iir-to-beam`/
`ir-to-beam` suites (including the new real-`erl` float tests) are the
executed evidence.

Dartmouth BASIC now declares 20/51 rows on `Beam` (up from 18/51).
Reprioritize after this merges: the next f64-lowering candidates are (a)
`neg`(f64) — needed by BASIC's `RND`/unary-minus expressions, structurally
trivial (mirrors `neg`(i64)'s existing `gc_bif1` shape with `erlang:'-'/1`,
already polymorphic over int/float); (b) `f64_pow` — needed by variable/
fractional `^` exponents, no direct single-BIF equivalent (`math:pow/2`
returns a float already, so likely a straightforward `gc_bif2`-shaped
`call_ext`, but unverified); (c) the still-open float-division-by-zero
design question, which should be raised explicitly (not guessed at) once a
row actually needs a runtime-variable divisor. VM-060b (BEAM host input,
unblocking BASIC's 5 `INPUT` rows and FLOW-MATIC's 4 together) and VM-041
(Twig dynamic-string/record/closure BEAM isolation) remain the other
standing non-ALGOL candidates.

## VM-040 Dartmouth BASIC BEAM pure-string family (selected after #15119 merged)

`git fetch origin && git merge origin/main` fast-forwarded cleanly to
`db7125e689` — no conflict, and `git log --oneline -30` confirmed both #15099
and #15119 (the two PRs that took all 58 COBOL-60 rows to full BEAM
declaration) already on main, matching this backlog's own top section.
`gh pr list --state open --limit 50` showed no LANG-VM-related open PR to
coordinate with.

**Real reprioritization, not another COBOL slice by habit.** With COBOL's
BEAM tail exhausted, this backlog's own top section explicitly calls for
reranking against the real remaining candidates rather than assuming more
COBOL rows exist. Ran the "Prioritization policy" order for real:

- **Rung 1/2 (red cells, missing CI protection):** none found. Every prior
  slice's trailing validation still reports positive sentinels; VM-024/032
  keep the matrix in normal CI.
- **Rung 3 (stale documentation):** re-audited `LANG-VM-FEATURE-COVERAGE.md`
  and `LANG-VM-FEATURE-COVERAGE.md`'s pinned `feature_coverage_doc_counts_match_programs_source`
  test against fresh source; both still agreed (the COBOL-60 completion had
  already been reconciled by #15119's own trailing update). No stale-doc gap
  found ahead of rung 4.
- **Rung 4 (missing backend parity):** this session's mandate specifically
  asked whether ANY other non-ALGOL frontend still has undeclared BEAM rows
  the way COBOL did before this effort — a question nobody had actually
  re-asked from source since VM-061 (which only reconciled Twig's *counted*
  cells, not audited every frontend's *Beam-declared* count). A brace/string-
  aware parse of every `Prog` entry's `lang`/`backends` fields, cross-checked
  against the compiled `feature_coverage_doc_counts_match_programs_source`
  test (which is the actual source of truth — a first pure-regex pass without
  string-escaping awareness overcounted several languages, including falsely
  reporting a `McCarthyLisp` row that does not exist), found the real
  per-frontend Beam-declared/total-rows split: Twig 20/49 (VM-041, design
  required), Nib 26/26 (complete), Brainfuck 3/6 (complete per VM-042 —
  the other 3 are the intentional `getchar` refusal), Oct 12/12 (complete),
  FlowMatic 4/8 (the other 4 are `INPUT`/EOF rows blocked on VM-060b),
  COBOL-60 58/58 (complete, this effort), and **Dartmouth BASIC 0/51** — the
  only non-ALGOL frontend with a *total* BEAM gap, despite being explicitly
  in scope since VM-040's original 2026-09 family list ("BASIC f64/I/O")
  and never picked up while every other family in that list got addressed.
  This is the closest analogue to the just-finished COBOL work: a large,
  completely untouched BEAM gap on an existing, fully-implemented frontend.

**Why BASIC's gap is bigger than COBOL's was, and why this slice does not
close all of it.** Before writing a single test, probed all 51 corpus rows
through `lang_aot::compile_source_to_beam` (compile-only, no `erl` needed for
this first pass). Every row failed except 18, all with the identical
`iir-to-beam` validation error: `"const instruction has a Float operand;
float constants are not supported"`, reported against `__basic_print_real`/
`__basic_print_fixed_mag`/`__basic_print_real_e` — helper functions the
frontend unconditionally emits into every compiled module regardless of
whether a given program's control flow ever reaches them. Reading
`iir-to-beam/src/lower.rs` end to end confirms it has genuinely zero `f64`
lowering support (no `f64`/`Float`/`fadd` match arm anywhere in the file,
versus full string-op coverage: `str_const`/`str_concat`/`str_slice`/
`str_len`/`str_index`/`str_eq`/`str_cmp` all already exist). Cross-referencing
`dartmouth-basic-iir-compiler`'s doc comments confirms this is not a narrow
gap: BA7-1b made *every* BASIC scalar numeric value — even an integer-spelled
literal like `PRINT 42` — ride the shared `f64` value track, so the float-op
gap blocks nearly the entire numeric two-thirds of the corpus, not an edge
case. This is qualitatively different from every prior VM-040 COBOL slice,
which only ever needed *promotion* (every required op already had a BEAM
lowering). Real BASIC-BEAM numeric parity needs new `iir-to-beam` production
code — a properly scoped design item in its own right, not a same-day
bounded slice — so it is intentionally NOT attempted here.

The exactly-18 rows that DO compile today are structurally distinct: every
one is purely string-valued (string literals/variables, `+` concatenation,
`=`/`<>`/`<`/`>` comparison, `IF`/`GOTO` control flow) with no numeric
`PRINT`/`LET`/`FOR`/`INPUT` anywhere in source, so the frontend never emits a
float `const` at all and the poisoned shared helpers are simply never
reached. This is a genuine, zero-new-lowering promotion opportunity — the
same shape as every COBOL BEAM slice — that nobody had noticed because
VM-040's original family list bucketed all of BASIC under one undifferentiated
"f64/I/O" line item.

### VM-040 Dartmouth BASIC BEAM pure-string family contract

Add `Beam` to the 18 consecutive pure-string `DartmouthBasic` rows' `backends`
arrays (filtered indices 2–19, i.e. `.skip(2).take(18)` — immediately after
the two numeric baseline rows and immediately before the first `FOR`/`NEXT`
loop) and add `portable_text_stdout_dartmouth_basic_beam_strings`, pinning
each row's expected stdout so an insertion cannot silently redirect the
proof. Execute all 18 against real `erl` before promoting — per this loop's
own "probe before declaring" discipline, even though the compile-only probe
already gave high confidence nothing new was needed. Do not touch the
remaining 33 numeric/`INPUT` rows; log the float-lowering gap and the
host-input gap as separate follow-up work rather than attempting either here.

### VM-040 Dartmouth BASIC BEAM pure-string family validation

All 18 selected programs passed on real Erlang
(`portable_text_stdout_dartmouth_basic_beam_strings`), byte-identical to
every other backend's stdout; no `iir-to-beam` defect was found and no
production code changed — a clean-pass promotion, exactly like Nib/Oct/
COBOL's prior BEAM slices. Two of the fresh cells (global indices 362 and
379, the first and last of the 18) were independently re-verified in fresh
processes via `LANG_MATRIX_ONLY_CELL`, each emitting its positive
`lang-matrix: single-cell ran` sentinel. All 22 BEAM-prefixed `lang_matrix`
tests pass together (21 prior plus this new one). `feature_coverage_doc_counts_match_programs_source`
was updated (Dartmouth BASIC tuple `(51, 357)` → `(51, 375)`) and passes
against the live `PROGRAMS` corpus. `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth
BASIC row and grand-total prose were updated to match (1593 → 1611 declared
cells). The full `non_algol_matrix_every_proven_cell_agrees` capstone
confirmed the corrected total against a live run: 210 programs, **1401**
cells exercised, 210 skipped (the same host-wide missing `ilasm` pattern
every prior slice reports), zero failures, in 602.62s — 1401 + 210 = 1611
matches the corrected declared total exactly. Dartmouth BASIC is no longer
the one non-ALGOL frontend with a total BEAM gap; its remaining 33 rows need
new `iir-to-beam` float lowering (numeric family) or the still-unscoped BEAM
host-input design (the 5 `INPUT` rows, shared with FLOW-MATIC's blocked
rows — VM-060b).

Reprioritize after this merges: no red cell or missing-CI-protection issue
was found ahead of this slice, and none is expected to appear ahead of the
next one either. The real remaining rung-4/rung-5 candidates are: (a) scope
and implement `iir-to-beam` f64 lowering as its own item, unlocking BASIC's
remaining ~28 numeric rows (a materially larger, genuinely novel-lowering
undertaking — likely its own multi-slice track, the way COBOL BEAM was);
(b) VM-041 (Twig dynamic-string/record/closure BEAM isolation, 29 rows,
design required); (c) VM-060b (BEAM host-input design, unblocking BASIC's 5
`INPUT` rows and FLOW-MATIC's 4 `INPUT`/EOF rows together); (d) VM-058 (COBOL
`BEFORE`/`AFTER` single-phrase intersection, 9 call sites, bounded but new
frontend semantics). Recommend scoping the BASIC f64-lowering item next: it
is the only one of the four that is bounded backend-parity work for an
already-fully-implemented, already-tested language feature (rung 4) rather
than new frontend semantics (rung 5), even though — unlike every prior VM-040
slice — it needs new production code before any row can promote.

## VM-040 COBOL BEAM regions and self-move (selected after #15099)

Refreshed main to `e8b737045b` after #15099 merged. External PR #15103
independently repeats the four base replacements already on main; it does
not cover the six rows selected here. Preserve that owner's branch.
Prioritize the remaining existing COBOL corpus gaps over new Twig/CLR host
ABI design and full Oct/Nib machine semantics: five INSPECT region cases
and the STRING self-move regression complete this declared corpus's BEAM
column. No observed failing cell currently takes precedence.

Contract: execute COBOL filtered indices 52–57 on real Erlang before
promotion. Pin source identities and exact stdout for STRING self-move,
BEFORE/AFTER tallying and replacing (including absent-delimiter asymmetry),
and combined tally-then-replace with independent regions. This does not
implement a single-phrase BEFORE/AFTER intersection (VM-058), nor establish
full COBOL language support. Missing Erlang may skip; detected-runtime
failure must fail. Promote only proven cells, run focused COBOL regression,
fresh individual matrix cells, coverage consistency and Clippy. Specify any
new backend defect before production edits.

Validation: six real Erlang probes passed before promotion; all thirteen
focused COBOL BEAM tests passed together. Fresh cells 469–474 emitted
positive execution sentinels, and coverage consistency plus all-target
lang-aot/iir-to-beam Clippy passed. No production fix was needed. All 58
COBOL corpus rows declare BEAM (464 cells); non-ALGOL declares 1593 cells.
No full-capstone rerun is claimed. After merge, reprioritize remaining
language/backend gaps and VM-058 rather than treating corpus completion as
full COBOL support.

## VM-040 remaining base INSPECT replacements (selected after #15033)

PR #15033 merged as `8f1c60fc1b` with 16 successful and 31 skipped checks.
Refreshed main and checked open PRs: ALGOL remains separately owned; no open
COBOL/BEAM implementation overlaps this slice. The ten missing COBOL BEAM
rows are four base replacements, five region cases and one STRING self-move.
Select the four base replacements before region coverage: they establish
LEADING, CHARACTERS, no rechaining and first matching clause precedence.
There is no observed failing cell ahead of them. Twig VM-041, encoded CLR
host input VM-060b and full Oct/Nib machine fidelity remain separate design
work; this bounded slice completes an existing replacement family first.

Contract: probe COBOL filtered indices 48 through 51 on real Erlang before
adding BEAM declarations. Require all four executions and exact existing
stdout, including padding and replacement order. Missing Erlang may skip;
a detected runtime failure must fail. Promote only these four rows after
successful probes, update coverage counts and documentation, and run the
coverage consistency test and focused Clippy. Any backend defect discovered
must be logged and specified before its implementation. No full machine
fidelity or rerun of the other seven backend columns is claimed.

Validation: all four probes passed before promotion. All twelve focused
COBOL BEAM tests then passed together, including pointer/overflow regressions.
Fresh matrix cells 463–466 each emitted their positive execution sentinel;
coverage consistency and all-target lang-aot/iir-to-beam Clippy passed.
No backend defect was found. COBOL now declares 52 BEAM rows and 458 cells;
the non-ALGOL total is 1587 declared cells. No full-capstone rerun is claimed.
Reprioritize the remaining five region rows and one self-move after merge.

## VM-040 COBOL BEAM pointer/overflow probe (selected after #15012 merged)

`git fetch origin && git merge origin/main` fast-forwarded cleanly from
`ef0d3910fb` to `092349e3ff` — one unrelated SwiftUI per-edge-borders commit
(UI79, `#15024`), confirmed by path to touch nothing under `lang-*`/`iir-*`/
`cobol-*`/BEAM or this backlog. `gh pr list --state open --limit 100` showed
one open PR (a stale `paint-vm-canvas` npm/yarn dependabot bump from July),
nothing LANG-VM-related, so nothing was in flight to coordinate with.

PR #15012's own trailing note recommended the pointer/overflow family (8
rows) as the next slice — the most structurally novel of the four remaining
COBOL BEAM families and therefore the best remaining chance to surface a
real `iir-to-beam` defect, following this backlog's own discipline of
probing before declaring. Re-derived the premise from source rather than
trusting the prior count: a brace-balanced scan of `PROGRAMS` restricted to
`lang: Language::Cobol60,` field lines (bounding each `Prog` by the next such
line, matching the approach that avoids fixed-window undercounting) found
exactly 58 `Cobol60` rows, 40 declaring `Beam` and 18 not. Printing each
row's filtered index alongside its `Beam` status confirmed the 8
pointer/overflow rows are exactly indices 36-43 — the eight rows immediately
preceding the base `INSPECT TALLYING`/`REPLACING` family PR #15012 just
promoted (indices 44-47 and beyond) — matching the prior slice's claim
exactly, not just approximately. Grepping this file for `VM-041` and
`VM-060b` still shows no dedicated scoped design entry for either beyond the
same one-liner in the "Ranked backlog" table; `VM-058` remains rung 5. No
LANG-VM-relevant commit landed between #15012 merging and this slice
starting (the fetch/merge above was a clean fast-forward with only the one
unrelated ALGOL/SwiftUI-adjacent UI commit), so the picture had not changed.

**Read `cobol-iir-compiler`'s pointer/overflow lowering end to end before
writing any test**, per this slice's specific mandate. `emit_string` handles
both the `DELIMITED BY SIZE` and `DELIMITED BY delim` cases; when a `WITH
POINTER p` phrase is present, both hand off to the shared
`emit_string_pointer_overlay` helper (also used by `emit_unstring`'s pointer
arm). That helper is genuinely novel in shape, not just op vocabulary: it
guards the pointer's range (`pv < 1` or `pv > W` jumps straight to `st_end`,
skipping everything below), then computes `start = pv - 1`, `avail = W -
start`, `cp = min(clen, avail)`, `end = start + cp`, and finally rebuilds the
receiver as `recv[0,start] ++ concat[0,cp] ++ recv[end,W]` via THREE
`str_slice` calls and TWO `str_concat` calls chained back to back in one
basic block — every bound a run-time value read from a live `PIC 9` pointer
item, not a compile-time constant. This is the first COBOL BEAM row to chain
multiple `str_slice`/`str_concat` calls through one helper over fully
run-time-computed bounds, rather than a single call with one run-time bound
(VM-D032's case) or a whole chain of compile-time-known ones (the earlier
STRING SIZE-delimited slices).

Traced the exact register flow to rule out a VM-057-shaped hazard before
probing: the receiver's register is read TWICE by this helper (once for the
head slice, once for the tail slice) and is not written until the FINAL
`str_concat` — by which point both reads have already landed in separate,
distinct registers (`headpre` and `tail`). Unlike VM-057's self-move row, the
receiver here is never both the source AND destination of the SAME op, so
`iir-to-beam`'s `str_slice`/`str_concat` lowerings (which always stage their
source operands into scratch registers before ever touching the destination)
have no aliasing hazard to hit regardless of `iir-to-beam`'s one-x-register-
per-name `reg_map` scheme. Also checked `str_slice`'s VM-D032-fixed bounds
check against the zero-length-slice edge case this helper can reach (`cp ==
0`, e.g. an `UNSTRING`/`STRING` whose concatenated content is itself empty):
the check's `end >= start` guard falls through (does not trap) when `end ==
start`, so an empty slice is accepted, matching every other backend's
contract. No op-level or register-flow gap was found by inspection alone —
consistent with the prior slice's audit — so this remained a probe-first
candidate, not a pre-diagnosed repair.

### VM-040 COBOL BEAM pointer/overflow contract

Add `Beam` to all 8 pointer/overflow rows' `backends` arrays (filtered
indices 36-43, `.skip(36).take(8)`) and
`portable_text_stdout_cobol_beam_pointer_overflow`, covering both `STRING
... WITH POINTER` (in-range, exact-fit, partial-transfer-then-overflow,
zero-pointer) and `UNSTRING ... WITH POINTER` (mid-field start, exhaustion,
past-end pointer, trailing-delimiter overflow) — exercising both the `ON
OVERFLOW` and `NOT ON OVERFLOW` branches of the shared dispatch skeleton.
Unlike the established four-row-at-a-time COBOL BEAM slices, this promotes
the full 8-row family in one slice: it is a naturally self-contained unit
(both verbs, both overflow outcomes) and the specific family this backlog
flagged as the best remaining candidate to expose a real defect, so bounding
it further would only delay the highest-value probe. Run against real `erl`
before treating the rows as proven, per this loop's own "probe before
declaring" discipline.

### VM-040 COBOL BEAM pointer/overflow validation

All eight programs passed on real Erlang on the first probe
(`portable_text_stdout_cobol_beam_pointer_overflow`); no `iir-to-beam` defect
was found and no production code changed — a clean-pass outcome, not a
repair, consistent with the source-level audit above (every op already
lowered for BEAM, no aliasing hazard, the bounds check already handles the
zero-length-slice edge case). `iir-to-beam`'s package suite is unaffected
(still 107 tests: 22 unit, 80 integration, 5 doc, all passing) since no
`iir-to-beam` source file changed. Focused Clippy on `lang-aot` and
`iir-to-beam` with all targets and warnings denied is clean. Forty-eight of
58 COBOL rows now declare BEAM (454 total declared cells, up from 446).
`feature_coverage_doc_counts_match_programs_source` was confirmed to
actually exercise the check by first running it against the pre-fix COBOL-60
figure of 446, where it failed with the expected assertion message naming
the 454/446 mismatch, before the doc was corrected to match
(`(Cobol60, 58, 446)` → `(Cobol60, 58, 454)`). `LANG-VM-FEATURE-COVERAGE.md`'s
COBOL-60 row and grand-total prose were updated to match (1575 → 1583
declared cells). The full `non_algol_matrix_every_proven_cell_agrees`
capstone confirmed the corrected total against a live run: 210 programs,
**1373** cells exercised (was 1365), 210 skipped (the same host-wide missing
`ilasm` pattern every prior slice reports), zero failures, in 490.62s —
matching the source-level audit's prediction exactly (eight newly exercised
cells, zero newly skipped, zero failures).

Reprioritize the remaining 10 undeclared COBOL BEAM rows (4 more base
INSPECT TALLYING/REPLACING, 5 VM-047c BEFORE/AFTER region, 1 VM-057
self-move) against VM-041 (Twig dynamic-string isolation), VM-060b (host
input design) and VM-058 (INSPECT BEFORE/AFTER intersection) after this
merges, following the same real policy comparison run above. None of the
three remaining COBOL families is a red cell or missing-CI-protection issue;
each is bounded, proof-promotion work with an established pattern and no
op-level novelty remaining (VM-047c's `emit_inspect_region_window` is a
strict superset of the base INSPECT family's already-proven op vocabulary;
VM-057's single self-move row was already checked safe by direct source
reading in an earlier slice). Absent a fresh audit finding otherwise, the
next slice should default to continuing the base INSPECT TALLYING/REPLACING
family's remaining four rows (REPLACING LEADING, REPLACING CHARACTERS,
REPLACING with no rechaining, REPLACING first-match) before VM-041/VM-060b/
VM-058, since bounded parity work with executable acceptance still
outranks open-ended new-semantics design per the prioritization policy.

## VM-040 COBOL BEAM INSPECT TALLYING/REPLACING probe (selected after #14999 merged)

`git fetch origin && git merge origin/main` reported "Already up to date" —
this worktree already sat at `9595613c41` with zero commits behind
`origin/main`, and PR #14999 (VM-040 COBOL BEAM UNSTRING/delimiter probe,
commit `1ee70e708d`) was directly visible at the tip of `git log --oneline
-5`. `gh pr list --state open --limit 100` returned zero rows, so nothing —
LANG-VM-related or otherwise — was in flight to coordinate with; no rebase or
conflict handling was needed to start this slice.

PR #14999's own trailing note made an important re-scoping: the remaining 22
undeclared COBOL BEAM rows are NOT one homogeneous family. A brace-balanced
scan restricted to the `lang: Language::X,` field lines confirmed the count
directly against source: 58 `Cobol60` rows, 36 declaring `Beam`, 22 not, and
— reading every undeclared row's source and its preceding comment — the 22
split exactly as the trailing note described: 8 pointer/overflow rows (lines
6139–6309: `STRING`/`UNSTRING` `WITH POINTER` and `ON OVERFLOW`/`NOT ON
OVERFLOW`), 8 base `INSPECT TALLYING`/`REPLACING` rows (lines 6333–6511:
`ALL`/`CHARACTERS`/`LEADING` tallying, `ALL`/`LEADING`/`CHARACTERS`/no-
rechaining/first-match replacing), 1 VM-057 STRING self-move row (line
6525), and 5 VM-047c `BEFORE`/`AFTER` region rows (lines 6548–6674). These
four families are contiguous in file order in exactly that sequence — the
22 undeclared rows are the literal tail of the 58 `Cobol60` rows, not
scattered through the array.

**Did anything change the picture since #14999 merged?** This worktree was
already at `origin/main`'s tip by construction, so no commit by anyone —
ALGOL's separately-owned campaign included — landed in any window to
re-examine. Grepping this file for `VM-041` and `VM-060b` still shows no
dedicated scoped design entry for either beyond the same one-liner in the
"Ranked backlog" table; nobody picked up either design task in the meantime,
and VM-058 remains rung 5 (new frontend semantics, not a red cell or missing
parity).

**Investigating each of the four families instead of assuming they carry
equal risk, per this session's specific mandate:**

- **Pointer/overflow (8 rows).** `cobol-iir-compiler::emit_string` (the
  `WITH POINTER` overlay path, `emit_string_pointer_overlay`) and
  `emit_unstring`'s pointer/overflow arm compile through `const`/`cmp_lt`/
  `cmp_gt`/`cmp_le`/`cmp_ge`/`sub`/`add`/`mov`/`str_slice`/`str_concat`/
  `jmp_if_true`/`jmp_if_false`/`jmp`/`label` — every one of those ops already
  has a BEAM lowering (`cmp_gt`/`cmp_le` confirmed present in
  `iir-to-beam/src/lower.rs`'s `cmp_*` match arm alongside `cmp_eq`/`cmp_ne`/
  `cmp_lt`/`cmp_ge`; `str_slice`'s bounds check was the exact defect VM-D032
  fixed two slices ago, so it now enforces its documented contract
  explicitly rather than relying on `lists:sublist`'s incidental leniency).
  This family is the most STRUCTURALLY novel of the four — the first COBOL
  BEAM rows to chain multiple `str_slice`/`str_concat` calls through one
  helper (`emit_string_pointer_overlay`) with fully run-time-computed
  bounds (the overlay offset comes from a live `PIC 9` pointer item, not a
  compile-time constant) — so it is the best candidate for exposing a real
  gap, even though no op-level gap was found by inspection alone.
- **Base INSPECT TALLYING/REPLACING (8 rows).** `emit_inspect_tallying`/
  `emit_inspect_replacing` (the region-less path) compile through
  `str_len`/`str_index`/`cmp_eq`/`cmp_ge`/`cmp_lt`/`const`/`add`/`sub`/`mov`/
  `jmp*`/`label`/`and`/`or`/`str_const` — confirmed by extracting every
  `self.emit("...")` op name from `cobol-iir-compiler::lib.rs` lines
  2713–4963 (the whole INSPECT family) via a small script rather than
  reading every line by eye. `str_len`/`str_index` were added two slices ago
  (VM-040 STRING SIZE/delimiter) and `and`/`or` were proven in the
  boolean/EVALUATE probe; nothing here is a novel combination. This is the
  lowest-risk family, exactly as this session's brief hinted: it is the
  closest in shape to already-proven work.
- **VM-047c BEFORE/AFTER region rows (5 rows).** These reuse
  `emit_inspect_region_window`, whose own op list is a strict SUBSET of the
  base family's (an extra delimiter-scan loop using the same `str_index`/
  `cmp_*`/`const`/`add`/`mov`/`jmp*`/`label` ops, no new op). The BEFORE/AFTER
  region semantics themselves were already fully implemented and
  exhaustively tested on all seven standard backends by VM-047c itself; this
  is purely a missing-BEAM-column promotion of already-proven behavior, not
  new design. Structurally as low-risk as the base family, just gated behind
  it in file order.
- **VM-057 STRING self-move edge case (1 row).** This row is isolated as its
  own family (rather than folded into the base STRING slices) because it is
  the exact regression that originally exposed the VM-057/VM-D032 defect
  class on WASM: `STRING S DELIMITED BY SIZE INTO S` lowers (via
  `cobol-iir-compiler::string_source`, which returns an `Operandy::Name`'s
  own live register directly) to a single `str_slice` whose destination and
  source are the IDENTICAL register. `iir-to-beam` shares WASM's general
  register-allocation SHAPE — `reg_map: HashMap<String, u8>` maps one
  x-register per variable NAME, the same "one mutable slot per name" pattern
  that made WASM vulnerable — so this row specifically deserved the
  aliasing-hazard scrutiny the task called for, not just an op-presence
  check. Reading `iir-to-beam/src/lower.rs`'s `str_slice` arm line by line:
  the bounds check reads `src_reg`/`start_reg`/`end_reg` directly (before
  anything is staged); then `src_reg`, `start_reg` and `end_reg` are each
  `OP_MOVE`d into scratch registers ABOVE `next_reg` (`s_src`/`s_start1`/
  `s_len`) BEFORE the `call_ext` to `lists:sublist/3` runs; only AFTER that
  call does `if rd != 0 { move x0 -> rd }` write the destination. Since
  `s_src` already captured the original value of `src_reg` before `rd` is
  ever touched, `rd == src_reg` (the exact self-move shape) is safe — the
  aliasing hazard that hit WASM's bump-allocate-then-`memory.copy` shape
  does NOT recur in BEAM's stage-then-call-then-writeback shape. This is a
  negative result specific to one row, not evidence the family is riskier
  than its size suggests; logged here so the next slice does not re-derive
  it from scratch.

**Conclusion:** all four families are rung-4 work (missing backend parity
for an already-implemented, already-tested COBOL feature); none is a red
cell, and no CI-protection or stale-documentation issue was found ahead of
them (rungs 1–3 clear, as every prior slice's trailing validation confirms).
`VM-041`/`VM-060b` remain rung 5 with no scoped design progress. Op-level
risk is effectively tied across all four — every required op already has a
multi-slice BEAM track record, and the one family flagged for a possible
aliasing gap (VM-057) checks out safe by direct source reading. Given that
tie, this slice selects the family the task's own investigation independently
converges on as safest to execute cleanly: **base INSPECT TALLYING/REPLACING**
— no op combination here is novel to BEAM, unlike pointer/overflow's
first-time chained-overlay shape. This is a deliberate DEVIATION from pure
file order (pointer/overflow's 8 rows precede this family in the array) —
justified by the investigation above, not assumed by habit. Pointer/overflow
is queued next specifically because its structural novelty makes it the best
remaining candidate to actually expose a defect, per rung-1's standing
priority over further easy parity work once a real candidate is identified.

### VM-040 COBOL BEAM INSPECT TALLYING/REPLACING contract

A bounded proof-promotion slice, following the established `.skip(N).take(4)`-
over-`Cobol60`-filter pattern directly: no new design decisions, since the
four target programs' compiled IIR uses no operation absent from
`iir-to-beam`. Add `Beam` to the four rows' `backends` arrays (`TALLYING FOR
ALL`, `TALLYING FOR CHARACTERS`, `TALLYING FOR LEADING`, `REPLACING ALL` —
filtered indices 44–47, i.e. `.skip(44).take(4)`, since the 8 undeclared
pointer/overflow rows occupy indices 36–43 ahead of this family and are
deliberately left for the next slice), add
`portable_text_stdout_cobol_beam_inspect_tallying_replacing` mirroring the
existing COBOL BEAM probe tests, and run it against real `erl` before
treating the rows as proven — per this loop's own "probe before declaring"
discipline, even though the op-level audit above gave high confidence
nothing new was needed.

### VM-040 COBOL BEAM INSPECT TALLYING/REPLACING validation

All four selected programs passed on real Erlang on the first probe
(`portable_text_stdout_cobol_beam_inspect_tallying_replacing`); no
`iir-to-beam` defect was found and no production code changed — the
op-level audit's prediction held. `iir-to-beam`'s package suite is
unaffected (still 107 tests: 22 unit, 80 integration, 5 doc, all passing)
since no `iir-to-beam` source file changed. All 18 BEAM `lang_matrix` tests
pass together (fourteen prior plus this new one), 15.03s. Focused Clippy on
`lang-aot` and `iir-to-beam` with all targets and warnings denied is clean.
Forty of 58 COBOL rows now declare BEAM (446 total declared cells, up from
442). `feature_coverage_doc_counts_match_programs_source` was updated (COBOL-
60 tuple `(58, 442)` → `(58, 446)`) and confirmed passing against the live
`PROGRAMS` corpus. The full `non_algol_matrix_every_proven_cell_agrees`
capstone confirmed the corrected total against a live run: 210 programs,
**1365** cells exercised (was 1361), 210 skipped (the same host-wide missing
`ilasm` pattern every prior slice reports), zero failures, in 496.04s —
matching the op-level audit's prediction exactly (four newly exercised
cells, zero newly skipped, zero failures).

Reprioritize the remaining 18 undeclared COBOL BEAM rows (8 pointer/overflow,
4 more base INSPECT TALLYING/REPLACING, 5 VM-047c BEFORE/AFTER region, 1
VM-057 self-move) against VM-041 (Twig dynamic-string isolation), VM-060b
(host input design) and VM-058 (INSPECT BEFORE/AFTER intersection) after this
merges, following the same real policy comparison run above. Pointer/overflow
is the natural next candidate per this slice's own reasoning (structurally
novel, best remaining chance to surface a real defect) unless a fresh audit
finds otherwise.

## VM-040 COBOL BEAM UNSTRING/delimiter probe (selected after #14983 merged)

`git fetch origin && git merge origin/main` reported "Already up to date" —
this worktree already sat at `32cf9a5efa` with zero commits behind
`origin/main`, and PR #14983 (VM-040 COBOL BEAM STRING SIZE/delimiter probe,
commit `b1335971d9`) was directly visible at the tip of `git log --oneline
-5`. `gh pr list --state open --limit 100` returned zero rows, so nothing —
LANG-VM-related or otherwise — was in flight to coordinate with; no rebase or
conflict handling was needed to start this slice.

PR #14983's own trailing note asked for a real reprioritization of the ~26
undeclared COBOL BEAM rows against VM-041 (Twig dynamic-string isolation),
VM-060b (host input design) and VM-058 (INSPECT BEFORE/AFTER intersection) —
the same comparison run eight times before this one. Re-checked against
current source rather than repeated by habit:

**Did anything change the picture since #14983 merged?** This worktree was
already at `origin/main`'s tip by construction (the fetch/merge above
reported "Already up to date"), so no commit by anyone — ALGOL's separately-
owned campaign included — landed in any window to re-examine. Grepping this
file for `VM-041` and `VM-060b` still shows no dedicated scoped design entry
for either beyond the same one-liner in the "Ranked backlog" table — nobody
picked up either design task in the meantime, and VM-058 remains rung 5
(new frontend semantics on top of an already-implemented BEFORE/AFTER
region pair, not a red cell or missing parity).

**Is the "~26 undeclared rows" figure still accurate?** Re-verified directly
against source: a brace-balanced scan restricted to the `lang: Language::X,`
field lines and bounded each `Prog` by the next such field line (rather than
a fixed line-count window, which under-scans COBOL rows whose `src` runs
past a hundred lines) found exactly 58 `Cobol60` rows, of which exactly 32
declare `Beam` and exactly 26 do not — the "~26" estimate was, again, exactly
right. Cross-checked against `feature_coverage_doc_counts_match_programs_source`'s
existing pinned tuple `(Cobol60, 58, 438)`, which matches 32 Beam-declaring
rows out of 50 non-Beam-declaring cells (7 standard backends × 26 = 182,
7 × 32 + 32 = 256, 182 + 256 = 438) — arithmetic-consistent with the direct
scan, not just a second independent count.

**Conclusion:** nothing outranks continuing the COBOL BEAM rows. Rungs 1–2
are clear (every trailing validation paragraph reports positive sentinels;
VM-024/VM-032 keep the matrix and Windows execution in normal CI). Rung 3
found nothing stale this round: `LANG-VM-FEATURE-COVERAGE.md`'s COBOL-60 row
and grand-total prose already agreed with source before this slice started
(438 declared cells, 1567 grand total, matching the pinned test tuple).
Between the rung-4 COBOL BEAM rows (bounded, proof-promotion, an established
`.skip(N).take(4)` pattern with nine successful prior slices) and rung-5
VM-041/VM-060b (still genuinely open-ended design work with no scoping
progress since last checked), COBOL BEAM rows win again — a legitimate,
re-verified outcome, not a repeated habit.

The next four rows in file order after the thirty-two already declared are:
one more `STRING ... DELIMITED BY delim` row (an item delimiter stops a
sender at its first match, even when another follows) and three `UNSTRING`
rows (truncating/padding fields with no remainder carried into the last
receiver; leading/consecutive delimiters producing space-filled empty
receivers; an item delimiter with source exhaustion leaving later receivers
untouched). All four compile through `cobol-iir-compiler`'s
`emit_prefix_before_delim`/`emit_unstring`, both of which read fields via
`str_len`/`str_index`/`str_slice`/`str_concat`/`cmp_*` — every one of those
ops already lowered for BEAM before this slice (the immediately prior slice
added `str_len`/`str_index`; everything else was already proven by earlier
COBOL BEAM rows). Read both compiler routines end to end before probing to
confirm this, rather than discovering an unsupported op via a failing probe.

### VM-040 COBOL BEAM UNSTRING/delimiter contract

A bounded proof-promotion slice, following the established `.skip(N).take(4)`-
over-`Cobol60`-filter pattern directly: no new design decisions, since the
four target programs' compiled IIR uses no operation absent from `iir-to-beam`.
Add `Beam` to the four rows' `backends` arrays, add
`portable_text_stdout_cobol_beam_unstring_and_delimiter` mirroring the
existing COBOL BEAM probe tests (`.skip(32).take(4)`), and run it against real
`erl` before treating the rows as proven — per this loop's own "probe before
declaring" discipline, even though the op-level audit above gave high
confidence nothing new was needed.

### VM-040 COBOL BEAM UNSTRING/delimiter validation

All four selected programs passed on real Erlang on the first probe
(`portable_text_stdout_cobol_beam_unstring_and_delimiter`); no `iir-to-beam`
defect was found and no production code changed — the op-level audit's
prediction held. `iir-to-beam`'s package suite is unaffected (still 107
tests: 22 unit, 80 integration, 5 doc, all passing) since no `iir-to-beam`
source file changed. Focused Clippy on `lang-aot` and `iir-to-beam` with all
targets and warnings denied is clean. Thirty-six of 58 COBOL rows now declare
BEAM (442 total declared cells, up from 438).
`feature_coverage_doc_counts_match_programs_source` was confirmed to actually
exercise the check by first running it against the pre-fix COBOL-60 figure of
438, where it failed with the expected assertion message naming the 442/438
mismatch, before the doc was corrected to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone confirmed the corrected
total against a live run: 210 programs, **1361** cells exercised (was 1357),
210 skipped (the same host-wide missing `ilasm` pattern every prior slice
reports), zero failures, in 482.89s — confirming the doc's corrected grand
total (1567 → 1571) against a live run rather than arithmetic alone.

Reprioritize the remaining ~22 undeclared COBOL BEAM rows against VM-041
(Twig dynamic-string isolation), VM-060b (host input design) and VM-058
(INSPECT BEFORE/AFTER intersection) after this merges, following the same
real policy comparison run above — re-verifying premises against source, not
repeating prior conclusions by habit. The remaining 22 rows are no longer a
single homogeneous family: 8 pointer/overflow rows (STRING/UNSTRING `WITH
POINTER` and `ON OVERFLOW`/`NOT ON OVERFLOW`), 8 base `INSPECT
TALLYING`/`REPLACING` rows (ALL/CHARACTERS/LEADING variants, first-match and
no-rechaining semantics), 5 VM-047c `BEFORE`/`AFTER` region rows (including
the combined-regions form), and 1 VM-057 STRING self-move edge case (a
sending field that is also its own INTO receiver). The next slice should
re-scope against this actual composition rather than assuming the pointer/
overflow shape carries through unchanged.

## VM-040 COBOL BEAM STRING SIZE/delimiter probe (selected after #14803 merged)

`git fetch origin && git merge origin/main` reported "Already up to date" —
this worktree already sat at `4bb9aecd21` (PR #14803, VM-040 COBOL BEAM
reference-modification MOVE/trap probe) with zero commits behind
`origin/main` (`git rev-list --count 4bb9aecd21..origin/main` = 15, every one
of them an ALGOL `feat(algol): unroll ... powers` commit — the separately-
owned campaign's own work, confirmed by path via `git log --oneline
4bb9aecd21..origin/main -- code/packages/rust/iir-to-beam code/packages/rust/ir-to-beam
code/packages/rust/cobol-iir-compiler code/packages/rust/cobol-runtime
code/packages/rust/lang-aot/tests/lang_matrix.rs code/specs/LANG-VM-NON-ALGOL-BACKLOG.md
code/specs/LANG-VM-FEATURE-COVERAGE.md` returning nothing). `gh pr list
--state open` showed six open PRs: a Compose table-wheel feature, three
npm/yarn dependabot bumps, one GitHub Actions dependabot bump, and one stale
`paint-vm-canvas` dependabot bump from July — none touching this backlog or
any `lang-*`/`iir-*`/`cobol-*`/BEAM path, so nothing was in flight to
coordinate with.

PR #14803's own trailing note asked for a real reprioritization of the ~30
undeclared COBOL BEAM rows against VM-041 (Twig dynamic-string isolation),
VM-060b (host input design) and VM-058 (INSPECT BEFORE/AFTER intersection) —
the same comparison run eight times now. Re-checked against current source
rather than repeated by habit:

**Did anything change the picture since #14803 merged?** The path-scoped
`git log` above returned nothing for any LANG-VM-relevant path; every commit
since was ALGOL's own `unroll ... powers` work (separately owned, untouched
here). Grepping this file for `VM-041` and `VM-060b` still shows no dedicated
scoped design entry for either beyond the same one-liner in the "Ranked
backlog" table — nobody picked up either design task in the meantime.

**Is the "~30 undeclared rows" figure still accurate?** Re-verified directly
against source with a brace-balanced parse restricted to `PROGRAMS: &[Prog] =
&[..]` (found via `];` at column 0, since Brainfuck source strings contain
literal `[`/`]` characters that break naive whole-file bracket-balancing).
The array itself grew from 455 to 470 total entries since the last count —
ALGOL's own `unroll ... powers` commits added 15 new ALGOL rows — but the
non-ALGOL total held exactly at 210, confirming those additions never touched
this backlog's scope. Of the 58 `Cobol60` rows, exactly 28 declare `Beam`
(matching every number pinned since the reference-modification MOVE/trap
slice) and exactly 30 do not — the "~30" estimate was, again, exactly right.

**Conclusion:** nothing outranks continuing the COBOL BEAM rows. Rungs 1–2
are clear (every trailing validation paragraph reports positive sentinels;
VM-024/VM-032 keep the matrix and Windows execution in normal CI). Rung 3
found nothing stale this round. Between the rung-4 COBOL BEAM rows (bounded,
proof-promotion, an established `.skip(N).take(4)` pattern with eight
successful prior slices) and rung-5 VM-041/VM-060b (still genuinely
open-ended design work with no scoping progress since last checked), COBOL
BEAM rows win again.

**Sweep for other `iir-to-beam` ops with the same "relies on incidental host
leniency" shape as VM-D032's `str_slice` gap:** the prompt driving this
session specifically asked whether any other `str_*`/`array_*` lowering
still relies on a host function's incidental leniency instead of its own
explicit documented contract, the way `str_slice` relied on `lists:sublist`'s
silent truncation before VM-D032's fix. Read every `str_*`/`array_*`/
`*_byte` lowering arm in `iir-to-beam/src/lower.rs`:

- `array_get`/`array_set`/`load_byte`/`store_byte` lower to
  `atomics:get/3`/`atomics:put/3`. `atomics` enforces a *strict* `1..size`
  index range and raises `badarg` outside it — stricter than, not more
  lenient than, the documented contract (`vm-core::dispatch::handle_array_get`
  et al.: trap on out-of-range or negative index). This is the OPPOSITE
  direction from `str_slice`'s gap (host too permissive); here the host's own
  enforcement already matches or exceeds the contract, so no explicit guard
  is missing. VM-042's prior real-`erl` corpus proof (three Brainfuck rows)
  already exercises this path without incident.
- `add`/`sub`/`mul`/`div`/`mod` lower to `erlang:+//-///div/rem`. Erlang's
  own `div`/`rem` raise `badarith` on division by zero — a real, documented
  trap, not incidental leniency — matching every other backend's div-by-zero
  contract.
- `field_load`/`field_store` (the LispyPair cons-cell substrate) validate
  the field index (0=head, 1=tail) at BUILD/compile time
  (`InvalidOperand` for any other literal index) — never a data-dependent
  runtime bounds question, so there is no host leniency to inherit at all.
- `str_len`/`str_index` did not exist in this backend before this slice (see
  below) — added as part of this contract, with the exact same "does the
  host's own failure mode match the contract" scrutiny `str_slice` got.

No other concrete gap of VM-D032's shape was found. This is a negative
result, not a new discovery — logged here rather than silently, per the
prompt's own request, so the next slice does not have to redo this sweep
from scratch.

The next four rows in file order after the twenty-eight already declared
(lines 5652, 5671, 5690, 5712, plus the four from the reference-modification
MOVE/trap slice) are: three `STRING ... DELIMITED BY SIZE` rows (item
padding/truncation into differently-sized receivers; a short-receiver
truncate paired with an exact-fill receiver in the same program; a changed
source re-read by a second `STRING` into the same receiver) and one
`STRING ... DELIMITED BY ","` row (cutting each sender at its first
delimiter, keeping an absent delimiter's field whole, preserving the
receiver's tail). The three `SIZE`-delimited rows compile entirely through
`str_concat`/`str_slice`/`const` — already proven by earlier COBOL BEAM
slices, including the pointer/overflow overlay helper's reuse of the same
three ops. The fourth row is the first COBOL BEAM row to reach a
`DELIMITED BY delim` (non-`SIZE`) sending-field scan: `cobol-iir-compiler`'s
`emit_prefix_before_delim` (shared by `STRING ... DELIMITED BY delim` and
`UNSTRING`) scans a field character-by-character with `str_len`/`str_index`/
`str_slice` — and `str_len`/`str_index` had no BEAM lowering at all before
this slice (confirmed: `grep` for either name in `iir-to-beam/src` returned
nothing). Run this slice (`.skip(28).take(4)`) on real Erlang, matching the
established discipline: probe before declaring, commit a bounded contract
for any newly exposed gap, promote only executed cells. Two `UNSTRING` rows
and one more `STRING ... DELIMITED BY delim` row immediately follow in file
order and share the identical `str_len`/`str_index` dependency; they are left
for the next slice rather than folded in here, keeping this one bounded to
exactly four rows like every prior slice.

### VM-040 COBOL BEAM `str_len`/`str_index` contract

Probing the fourth row before declaring it found — as expected from the
`grep` above, not as a surprise — that `iir-to-beam` has no lowering for
`str_len` or `str_index` at all: `validate_for_beam` does not reject either
(neither is in `UNSUPPORTED_OPS`, and neither carries the `"str"` type_hint
the ASCII-string-subset check restricts, since both produce an `i64` result),
so a module using them passes validation and then fails cleanly at
`lower_iir_to_beam` with the crate's own exhaustive-match fallback
(`UnsupportedOp`, naming the op) — never silently.

Add both, reusing already-established patterns rather than inventing new
ones:

- `str_len` lowers to `erlang:length/1` via `gc_bif1` — the exact same
  recognized-guard-BIF pattern `neg`/`not` and `str_slice`'s own bounds check
  already use, and the same `import_length` `str_slice` already registered.
  No bounds concern: length has no failure mode.
- `str_index` lowers `idx` to `lists:nth(idx + 1, source)` via `call_ext`
  (new `lists:nth/2` import, staged through scratch registers above
  `next_reg` before the call, mirroring `load_byte`/`array_get`'s existing
  parallel-move-hazard discipline). Checked against `str_index`'s documented
  contract (`vm-core::dispatch::handle_str_index`: trap when `idx < 0 ||
  idx >= length(source)`) BEFORE writing the lowering, the same way
  `str_slice`'s gap was found: `lists:nth(N, List)`'s own two clauses
  (`nth(1, [H|_]) -> H;` and `nth(N, [_|T]) when N > 1 -> nth(N-1, T)`) have
  no case for `N =< 0`, and recursing past the list's end always lands on
  `nth(K, [])` for some `K >= 1`, which matches neither clause either — both
  out-of-range directions already raise `function_clause` on their own. So,
  unlike `str_slice`, no separate explicit guard is needed: the 0-based →
  1-based `+1` conversion IS what makes the host's native failure mode line
  up exactly with the documented contract. Add both `str_len`/`str_index` to
  the validator's `InvalidString` shape checks (dest + one/two string-typed
  operands + integer result), matching `str_eq`/`str_cmp`'s existing pattern.

Add four new real-`erl` regressions pinning this directly: `test_75` (basic
`str_len` correctness), `test_76` (basic `str_index` correctness), and the
same trap/boundary-control pair shape `test_73`/`test_74` established for
`str_slice` — `test_77` (`idx == length(source)`, one past the last valid
index, must trap) and `test_78` (`idx == length(source) - 1`, the last valid
index, must NOT trap, proving the boundary is `idx >= len` not `idx > len`).

### VM-040 COBOL BEAM STRING SIZE/delimiter validation

All four selected programs passed on real Erlang on the first probe after
adding `str_len`/`str_index`
(`portable_text_stdout_cobol_beam_string_size_and_delimiter`). `iir-to-beam`'s
package suite grew from 100 to 107 tests (22 unit — three new validator
shape tests — 80 integration — four new real-`erl` regressions — 5 doc), all
passing; focused Clippy on `lang-aot` and `iir-to-beam` with all targets and
warnings denied is clean. All sixteen BEAM `lang_matrix` tests pass together
(fifteen prior + the new one — Oct, Nib, Brainfuck, FLOW-MATIC and now
thirty-two of 58 COBOL rows across eight probe batches, plus the immediate-
arithmetic/comparison/`putchar`-state regressions and the Brainfuck
frontend/backend refusal split), 14.30s. Thirty-two of 58 COBOL rows now
declare BEAM (438 total declared cells, up from 434).
`feature_coverage_doc_counts_match_programs_source` was confirmed to
actually exercise the check by first running it against the pre-fix COBOL-60
figure of 434, where it failed with the expected assertion message naming
the 438/434 mismatch, before the doc was corrected to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone passed: 210 programs,
**1357** cells exercised (was 1353), 210 skipped (the same host-wide missing
`ilasm` pattern every prior slice reports), zero failures, in 478.98s —
confirming the doc's corrected grand total (1563 → 1567) against a live run
rather than arithmetic alone.

Reprioritize the remaining ~26 undeclared COBOL BEAM rows against VM-041
(Twig dynamic-string isolation), VM-060b (host input design) and VM-058
(INSPECT BEFORE/AFTER intersection) after this merges, following the same
real policy comparison run above — re-verifying premises against source, not
repeating prior conclusions by habit. The next four rows in file order (two
`UNSTRING` rows and one more `STRING ... DELIMITED BY delim` row, all sharing
this slice's `str_len`/`str_index` dependency, plus one more row past them)
are the natural continuation and already have their BEAM lowering primitives
in place.

## VM-040 COBOL BEAM reference-modification MOVE/trap probe (selected after #14784 merged)

`git fetch origin && git merge origin/main` reported "Already up to date" —
this worktree already sat at `dd87a55d80` (PR #14784, VM-040 COBOL BEAM
string ops/reference modification probe) with zero commits behind
`origin/main` (`git rev-list --count dd87a55d80..origin/main` = 4, all four
unrelated to this backlog: a toolkit appearance fix, a task-app PNG-render
feature, an engram-app Qt path fix, and a lessons-doc entry). `gh pr list
--state open` showed six open PRs: a Compose directional-padding draft,
three npm/yarn dependabot bumps, one GitHub Actions dependabot bump, and one
stale `paint-vm-canvas` dependabot bump from July — none touching this
backlog or any `lang-*`/`iir-*`/`cobol-*`/BEAM path, so nothing was in flight
to coordinate with.

PR #14784's own trailing note asked for a real reprioritization of the ~34
undeclared COBOL BEAM rows against VM-041 (Twig dynamic-string isolation),
VM-060b (host input design) and VM-058 (INSPECT BEFORE/AFTER intersection).
Re-checked against current source rather than repeated by habit:

**Did anything change the picture since #14784 merged?** `git log --oneline
dd87a55d80..origin/main -- code/packages/rust/iir-to-beam code/packages/rust/ir-to-beam
code/packages/rust/cobol-iir-compiler code/packages/rust/cobol-runtime
code/packages/rust/lang-aot/tests/lang_matrix.rs code/specs/LANG-VM-NON-ALGOL-BACKLOG.md
code/specs/LANG-VM-FEATURE-COVERAGE.md` returned nothing — no commit by
anyone touched any of these paths since the last slice merged. Grepping this
file for `VM-041` and `VM-060b` still shows no dedicated scoped design entry
for either beyond the same one-liner in the "Ranked backlog" table — nobody
picked up either design task in the meantime.

**Is the "~34 undeclared rows" figure still accurate?** Re-verified directly
against source: a brace-balanced parse restricted to `PROGRAMS: &[Prog] =
&[..]` (lines 171–6530) produced exactly 455 entries again — 210 non-ALGOL
rows. Of the 58 `Cobol60` rows, exactly 24 declare `Beam` (matching every
number pinned since the string ops/refmod slice) and exactly 34 do not — the
"~34" estimate was, again, exactly right.

**Conclusion:** nothing outranks continuing the COBOL BEAM rows. Rungs 1–2
are clear (every trailing validation paragraph reports positive sentinels;
VM-024/VM-032 keep the matrix and Windows execution in normal CI). Rung 3
found nothing stale this round. Between the rung-4 COBOL BEAM rows (bounded,
proof-promotion, an established `.skip(N).take(4)` pattern with seven
successful prior slices) and rung-5 VM-041/VM-060b (still genuinely
open-ended design work with no scoping progress since last checked), COBOL
BEAM rows win again.

The next four rows in file order after the twenty-four already declared
(lines 5652, 5671, 5690, 5712) are: a constant reference-modification MOVE
that pads and truncates into differently-sized receivers (`WS(2:2)`/`WS(2:4)`
of a 5-character source into a 5-wide and a 2-wide receiver), a runtime MOVE
that refits using a live computed slice length (`WS(J:K)` with `K` reassigned
between two MOVEs), and two computed reference modifications that must fail
closed — a runtime `end` past the item's width (`WS(4:5)` on a 5-character
item) and a runtime `start` of zero after the 1-based → 0-based conversion
(`WS(0:2)`). The first two reuse `str_slice`/MOVE-with-padding, already
proven by the earlier COBOL BEAM alphanumeric MOVE/comparison slice. The
last two are the first COBOL BEAM rows to reach `Expect::Trap` — no COBOL
BEAM row, and no other language's BEAM row, had exercised a trapping
construct before this slice.

### VM-040 discovery: VM-D032 — `iir-to-beam`'s `str_slice` never enforced its own documented bounds-check contract

Probing the two trap rows before declaring them (per this loop's own
discipline) found a real defect rather than an already-solved case. A
scratch integration test lowered both trap programs through
`lang_aot::compile_source_to_beam` and ran the result on real `erl`,
unmodified:

- The `start`-past-zero row (`WS(J:2)` with `J=0`) DID trap — but only by
  accident: `lists:sublist(List, Start, Len)`'s own guard clauses require
  `Start >= 1`, so `Start=0` hits no matching clause and Erlang raises a
  `function_clause` error. This is a side effect of `sublist`'s internal
  structure, not a deliberate bounds check.
- The `end`-past-width row (`WS(4:5)` on a 5-character `"ABCDE"`, so
  `[start0=3, end=8)`) did NOT trap: `lists:sublist("ABCDE", 4, 5)` (1-based
  `Start=4`, `Len=5`) has `Start` in range, so `sublist` just returns as much
  as it can — `"DE"` (2 characters) — silently, with a zero exit code. This
  contradicts the documented `str_slice` contract every other backend
  enforces (`vm-core::dispatch::handle_str_slice`: trap when `start < 0 ||
  end < start || end > len`) — `iir-to-beam`'s `str_slice` lowering had
  never implemented that check itself; it relied entirely on `sublist`'s
  incidental leniency, which only happens to match the contract for some
  inputs and silently diverges for others. No matrix row had reached this
  input shape before (every earlier `str_slice` row was in-bounds), so
  nothing in CI had ever exercised the gap.

### VM-040 repair contract

Make `str_slice`'s BEAM lowering enforce the documented bounds-check
predicate explicitly, before `lists:sublist` ever runs: compute
`length(source)` via `erlang:length/1` (a recognized guard BIF, callable
through the same `gc_bif1` pattern `neg`/`not` already use), then check
`start < 0`, `end < start`, and `end > length` with `is_ge` conditional
branches — reusing the same synthetic-label-allocation pattern `cmp_*`
already established — jumping to a shared trap block on any violation that
calls `erlang:error(badarg)` (`call_ext` to `erlang:error/1`, which
unconditionally raises) before falling through to the existing `sublist`
call on the in-bounds path. Add direct `iir-to-beam`-level regressions for
both the exact silent-truncation defect (`end` past length, must now trap)
and its boundary control (`end == length`, exactly full-length, must NOT
trap — proving `end > len`, not an off-by-one `end >= len`). Extend
`run_beam` in `lang_matrix.rs` with the same `Expect::Trap` handling every
other backend's runner already has (a nonzero `erl` exit becomes
`RunResult::Trapped` when the program expects a trap, `cell_failed`
otherwise) — this is a harness gap, not a semantic change, matching the
family of prior BEAM-runner-observability fixes (e.g. the very first VM-040
probe's discarded-stdout fix). Anchor `erl`'s working directory in the
disposable temp dir already used for the `.beam` file, so a legitimately-
trapping cell's `erl_crash.dump` does not land in the repo checkout. Do not
change `cobol-iir-compiler` or any other frontend — the defect and its fix
are entirely within `iir-to-beam`'s `str_slice` lowering.

### VM-040 COBOL BEAM reference-modification MOVE/trap validation

After the repair, all four selected programs passed on real Erlang
(`portable_text_stdout_cobol_beam_refmod_move_and_trap`). Two new
`iir-to-beam` regressions pin the bounds-check contract directly:
`test_73_real_erl_str_slice_traps_end_past_length` (the exact silent-
truncation case — `[1, 8)` of `"ABCDE"`, previously returned `"BCDE"`
silently, now traps) and `test_74_real_erl_str_slice_end_equal_to_length_is_in_bounds`
(`[0, 5)` of `"ABCDE"`, the exact boundary, must NOT trap). Both pre-existing
`str_slice` regressions (`test_70`, `test_71`) still pass unchanged. All
fifteen BEAM `lang_matrix` tests pass together (14 prior + the new one — Oct,
Nib, Brainfuck, FLOW-MATIC and now twenty-eight of 58 COBOL rows across seven
probe batches, plus the immediate-arithmetic/comparison/`putchar`-state
regressions and the Brainfuck frontend/backend refusal split), 13.11s.
`iir-to-beam`'s package suite grew from 98 to 100 tests (19 unit, 76
integration, 5 doc — the two new regressions), all passing; focused Clippy on
`lang-aot` and `iir-to-beam` with all targets and warnings denied is clean.
Twenty-eight of 58 COBOL rows now declare BEAM (434 total declared cells, up
from 430). `feature_coverage_doc_counts_match_programs_source` was confirmed
to actually exercise the check by first running it against the pre-fix
COBOL-60 figure of 430, where it failed with the expected assertion message
naming the 434/430 mismatch, before the doc was corrected to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone passed: 210 programs,
**1353** cells exercised (was 1349), 210 skipped (the same host-wide missing
`ilasm` pattern every prior slice reports), zero failures, in 511.43s —
exactly the four newly-promoted COBOL cells accounted for, confirming the
doc's corrected grand total (1559 → 1563) against a live run rather than
arithmetic alone.

Reprioritize the remaining ~30 undeclared COBOL BEAM rows against VM-041
(Twig dynamic-string isolation), VM-060b (host input design) and VM-058
(INSPECT BEFORE/AFTER intersection) after this merges, following the same
real policy comparison run above — re-verifying premises against source, not
repeating prior conclusions by habit.

## VM-040 COBOL BEAM string ops/reference modification probe (selected after #14779 merged)

`git fetch origin && git merge origin/main` reported "Already up to date" —
this worktree already sat at `d68f9bc3cb` (PR #14779, VM-040 COBOL BEAM
boolean/EVALUATE probe) with zero commits to fast-forward, confirming that
PR's own merge state directly (`git rev-list --count d68f9bc3cb..origin/main`
= 0). `gh pr list --state open` showed seven open PRs: a Qt host-control-
styles fix, a Compose directional-padding draft, four dependabot bumps
(npm/yarn ×3, GitHub Actions), and nothing else — none touching this backlog
or any `lang-*`/`iir-*`/`cobol-*`/BEAM path, so nothing was in flight to
coordinate with.

PR #14779's own trailing note asked for a real reprioritization of the ~38
undeclared COBOL BEAM rows against VM-041 (Twig dynamic-string isolation),
VM-060b (host input design) and VM-058 (INSPECT BEFORE/AFTER intersection).
Re-checked against current source rather than repeated by habit, per this
session's own mandate, specifically asking whether anyone had scoped VM-041
or VM-060b with an actual design since they were last checked:

**Did anything change the picture since #14779 merged?** `git log --oneline
d68f9bc3cb..origin/main -- code/packages/rust/iir-to-beam code/packages/rust/ir-to-beam
code/packages/rust/cobol-iir-compiler code/packages/rust/cobol-runtime
code/packages/rust/lang-aot/tests/lang_matrix.rs` returned nothing — the
worktree was already at `origin/main`'s tip, so by construction no commit by
anyone landed in this window. `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`
itself (grepped for `VM-041` and `VM-060b`) still shows no dedicated scoped
entry for either beyond the same "still has no scoped backlog entry"
language repeated across every prior slice back through VM-040's condition-
name/EVALUATE probe — nobody has picked up either design task in the
meantime. So VM-058's rung-5 status and VM-041/VM-060b's unscoped-design
status are not stale; there was nothing new to find.

**Is the "~38 undeclared rows" figure still accurate?** Re-verified directly
against source, the same way the prior two slices did rather than trusting
the carried-forward estimate. The same brace-balanced parse restricted to
`PROGRAMS: &[Prog] = &[..]` specifically (lines 171–6530) produced exactly
455 entries again — 210 non-ALGOL rows, matching every number pinned since
VM-061. Of the 58 `Cobol60` rows, exactly 20 declare `Beam` (the sixteen from
before #14779 plus that PR's own four) and exactly 38 do not — the "~38"
estimate was, again, exactly right.

**Conclusion:** nothing outranks continuing the COBOL BEAM rows. Rungs 1–2
are clear (every trailing validation paragraph back through VM-046c reports
positive sentinels; VM-024/VM-032 keep the matrix and Windows execution in
normal CI). Rung 3 found nothing stale this round. Between the rung-4 COBOL
BEAM rows (bounded, proof-promotion, an established `.skip(N).take(4)`
pattern with six successful prior slices) and rung-5 VM-041/VM-060b (still
genuinely open-ended design work with no scoping progress since last
checked), COBOL BEAM rows win again — this time's re-verification found the
same answer as the last several, which the prompt driving this session
explicitly flagged as a legitimate, non-suspicious outcome as long as the
check is real.

The next four rows in file order after the twenty already declared (lines
5652, 5671, 5690, 5712) are: an alphanumeric `EVALUATE GRADE` subject with a
`WHEN "A" THRU "M"` range (the first COBOL BEAM row to fold a `str_cmp`-
derived range with `and`, reusing `str_cmp`'s lexical ordering from the
initial-output slice and the boolean-fold machinery from the boolean/
EVALUATE slice), and three COBOL reference-modification rows — literal
bounds with an omitted length (`WS(2:3)`, `WS(3:)`, single-character
bounds), live computed indices (`COMPUTE`-derived start/length feeding
`DISPLAY`), and computed slices driving both an `IF` comparison and an
`EVALUATE` subject — all built entirely on `str_slice`, already proven by
the earlier COBOL BEAM alphanumeric MOVE/comparison slice (the `str_slice`
contract). No new opcode expected: this slice reuses `str_cmp` and
`str_slice`, both already accepted by `iir-to-beam`'s validator. Run this
slice (`.skip(20).take(4)`) on real Erlang, matching the established
discipline: probe before declaring, commit a bounded contract for any newly
exposed defect, promote only executed cells.

### VM-040 COBOL BEAM string ops/reference modification validation

All four selected programs passed on real Erlang on the first probe
(`portable_text_stdout_cobol_beam_string_ops_and_refmod`), with no new
`iir-to-beam`/`ir-to-beam` defect: each reuses `str_cmp`/`str_slice`/`and`/
branch lowering already proven by earlier COBOL BEAM rows, so no production
code changed this slice. All fourteen BEAM `lang_matrix` tests pass together
(13 prior + the new one — Oct, Nib, Brainfuck, FLOW-MATIC and now
twenty-four of 58 COBOL rows across six probe batches, plus the immediate-
arithmetic/comparison/`putchar`-state regressions and the Brainfuck
frontend/backend refusal split), 13.02s. `iir-to-beam`'s package suite is
unchanged at 98 tests (19 unit, 74 integration, 5 doc) since no production
code in that crate changed; focused Clippy on `lang-aot` and `iir-to-beam`
with all targets and warnings denied is clean. Twenty-four of 58 COBOL rows
now declare BEAM (430 total declared cells, up from 426).
`feature_coverage_doc_counts_match_programs_source` was confirmed to
actually exercise the check by first running it against the pre-fix COBOL-60
figure of 426, where it failed with the expected assertion message naming
the 430/426 mismatch, before the doc was corrected to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone passed: 210 programs,
**1349** cells exercised (was 1345), 210 skipped (the same host-wide missing
`ilasm` pattern every prior slice reports), zero failures, in 493.28s —
exactly the four newly-promoted COBOL cells accounted for, confirming the
doc's corrected grand total (1555 → 1559) against a live run rather than
arithmetic alone.

Reprioritize the remaining ~34 undeclared COBOL BEAM rows against VM-041
(Twig dynamic-string isolation), VM-060b (host input design) and VM-058
(INSPECT BEFORE/AFTER intersection) after this merges, following the same
real policy comparison run above — re-verifying premises against source,
not repeating prior conclusions by habit.

## VM-040 COBOL BEAM boolean/EVALUATE probe (selected after #14770 merged)

`git fetch origin && git merge origin/main` found this worktree already sat at
`4dfa11af8d` (PR #14770, VM-042 Brainfuck BEAM parity correction) with nothing
to fast-forward — that PR's own merge state is directly confirmed. `gh pr list
--state open` showed ten open PRs: Qt/Compose/toolkit UI feature work, a
Punjabi/Urdu human-languages PR, a CI shard-balancing fix, one Compose draft,
and four dependabot bumps (npm/yarn ×3, GitHub Actions) — none touching this
backlog or any `lang-*`/`iir-*`/`cobol-*`/BEAM path, so nothing was in flight
to coordinate with.

VM-042's own trailing note asked for a real reprioritization of the ~42
undeclared COBOL BEAM rows against VM-041 (Twig dynamic-string isolation),
VM-060b (host input design) and VM-058 (INSPECT BEFORE/AFTER intersection),
restating that VM-058 was re-examined and confirmed to stay at rung 5, and
that VM-041/VM-060b still have no scoped backlog entry. Rather than repeat
that conclusion by habit, it was re-checked against current source, per this
session's own mandate:

**Did anything change the picture since VM-042 merged?** `git log --oneline
4dfa11af8d..origin/main -- code/packages/rust/iir-to-beam code/packages/rust/ir-to-beam
code/packages/rust/cobol-iir-compiler code/packages/rust/cobol-runtime
code/packages/rust/lang-aot/tests/lang_matrix.rs` returned nothing — no commit
by anyone touched any of these paths between VM-042's merge and this session's
start, despite the repo's heavy concurrent activity (the fifteen most recent
commits on `main` are all unrelated: vision, mosaic, human-languages and
`adj-facts-stdlib` work). So VM-058's rung-5 status and VM-041/VM-060b's
unscoped-design status are not stale; there was nothing new to find there.

**Is the "~42 undeclared rows" figure still accurate?** Checked directly
against source rather than trusted, per VM-061's own lesson that stated
cell/row counts have been wrong before. A first naive whole-file
brace-balanced parse of every `Prog { .. }` literal in `lang_matrix.rs`
over-counted: it picked up a per-iteration differential-test helper
(`t7_differential_random_u8_expressions_agree`, which constructs a `Prog`
with `backends: &[]` inside a loop, reusing the same struct literal shape)
and a boundary-assertion `Prog` local to
`portable_text_stdout_preserves_content_and_brainfuck_bytes` (a
one-off `Language::Brainfuck` `Prog` used only to prove text-normalization
does not apply to Brainfuck's byte output) — neither is a row in the actual
`PROGRAMS` corpus. Restricting the parse to the `PROGRAMS: &[Prog] = &[..]`
static array specifically (lines 171–6530, found via `];` at column 0, since
Brainfuck source strings contain literal `[`/`]` characters that break naive
bracket-balancing across the whole file) produced exactly 455 entries — 210
non-ALGOL rows, 1551 non-ALGOL declared cells, matching every number VM-061
and VM-042 already pinned, including Twig's corrected 363 and Brainfuck's
corrected 45. Against that trustworthy parse: 58 `Cobol60` rows, exactly 16
declaring `Beam` and exactly 42 not — the "~42" estimate was, this time,
exactly right.

**Conclusion:** nothing outranks continuing the COBOL BEAM rows. Rungs 1–2
are clear (every trailing validation paragraph back through VM-046c reports
positive sentinels; VM-024/VM-032 keep the matrix and Windows execution in
normal CI). Rung 3 found nothing stale this round — VM-058, VM-041 and
VM-060b's statuses are all confirmed current, not just repeated. Between the
rung-4 COBOL BEAM rows (bounded, proof-promotion, an established
`.skip(N).take(4)` pattern with five successful prior slices) and rung-5
VM-041/VM-060b (still genuinely open-ended design work), COBOL BEAM rows win
again — this time on a freshly re-verified premise, not an assumed one.

The next four rows in file order after the sixteen already declared (lines
5568, 5587, 5606, 5629) are: a compound `(N > 1 OR N > 9) AND N < 8`
condition (folding leaf `cmp_*` booleans with bitwise `and`/`or`), `NOT (N <
3 OR N > 9)` (the first COBOL BEAM row to emit `xor`), an `EVALUATE` case
statement (`cmp_eq` + branch cascade, the same ops `IF` already uses), and an
`EVALUATE` with a multi-value/THRU-range `WHEN` (OR-folded `cmp_eq` and
`and(cmp_ge, cmp_le)`, the level-88 ranges machinery). All four are
boolean/comparison/branch lowering reusing already-BEAM-proven ops — no new
opcode expected, matching the family the prior condition-name/EVALUATE slice
already established. Run this slice (`.skip(16).take(4)`) on real Erlang,
matching the established discipline: probe before declaring, commit a
bounded contract for any newly exposed defect, promote only executed cells.

### VM-040 COBOL BEAM boolean/EVALUATE validation

All four selected programs passed on real Erlang on the first probe
(`portable_text_stdout_cobol_beam_boolean_and_evaluate`), with no new
`iir-to-beam`/`ir-to-beam` defect: each reuses `cmp_*`/`and`/`or`/`xor`/
branch lowering already proven by earlier COBOL BEAM rows, so no production
code changed this slice. All thirteen BEAM `lang_matrix` tests pass together
(12 prior + the new one — Oct, Nib, Brainfuck, FLOW-MATIC and now twenty of
58 COBOL rows across five probe batches, plus the immediate-arithmetic/
comparison/`putchar`-state regressions and the Brainfuck frontend/backend
refusal split), 12.43s. `iir-to-beam`'s package suite is unchanged at 98
tests (19 unit, 74 integration, 5 doc) since no production code in that
crate changed; focused Clippy on `lang-aot` and `iir-to-beam` with all
targets and warnings denied is clean. Twenty of 58 COBOL rows now declare
BEAM (426 total declared cells, up from 422).
`feature_coverage_doc_counts_match_programs_source` was confirmed to
actually exercise the check by first running it against the pre-fix COBOL-60
figure of 422, where it failed with the expected assertion message naming
the 426/422 mismatch, before the doc was corrected to match. The full
`non_algol_matrix_every_proven_cell_agrees` capstone passed: 210 programs,
**1345** cells exercised (was 1341), 210 skipped (the same host-wide missing
`ilasm` pattern every prior slice reports), zero failures, in 516.04s —
exactly the four newly-promoted COBOL cells accounted for, confirming the
doc's corrected grand total (1551 → 1555) against a live run rather than
arithmetic alone.

Reprioritize the remaining ~38 undeclared COBOL BEAM rows against VM-041
(Twig dynamic-string isolation), VM-060b (host input design) and VM-058
(INSPECT BEFORE/AFTER intersection) after this merges, following the same
real policy comparison run above — re-verifying premises against source,
not repeating prior conclusions by habit.

## VM-042 Brainfuck BEAM parity correction (selected after #14723 merged)

`git fetch origin && git merge origin/main` fast-forwarded this worktree to
`334182bc76` ("spec(UI47): design the effect-handler hook") with no conflicts;
the prior head was `04d9e16ac9` (PR #14723, VM-061), confirming that PR's own
merge state directly. `gh pr list --state open` showed six open PRs: a
Compose directional-padding feature and five dependabot bumps (npm/yarn ×3,
GitHub Actions), none touching this backlog or any LANG-VM path — nothing was
in flight to coordinate with or wait on.

VM-061's own trailing note asked for a real reprioritization of the ~42
undeclared COBOL BEAM rows against VM-041 (Twig dynamic-string isolation),
VM-060b (host input design), VM-042 (Brainfuck's BEAM-refusal pin) and VM-058
(the INSPECT BEFORE/AFTER two-delimiter intersection defect) — the same four
candidates every recent slice has compared, with COBOL BEAM rows winning each
time on the same structural reasoning (rung 4, bounded proof-promotion vs.
rung 5, open-ended design). Re-running the policy for real rather than
repeating that conclusion by habit:

**Rungs 1–2 (red cells / missing CI protection):** unchanged — every trailing
validation paragraph back through VM-046c reports positive sentinels, and
VM-024/VM-032 keep the matrix and Windows execution in normal CI.

**VM-058, re-examined against rung 1 specifically** (the session's own
mandate: don't trust the label already on it). VM-058 IS a genuine defect —
both `cobol-runtime`'s oracle and `cobol-iir-compiler`'s nine
`inspect_region` call sites silently read only the first of two grammar-legal
sibling nodes when one delimiter phrase carries both `BEFORE` and `AFTER` —
and its own discovery text (VM-D027) literally says "silently discarding a
second [keyword]", language that echoes rung 1's "silent backend skip"
closely enough to warrant checking. But rung 1 is specifically about a
*currently executed* red cell or a skip that is masking one — and neither
applies here: no matrix row exercises this two-delimiter construct (VM-047c
deliberately scoped it out), and the regression VM-047c added pins the
CURRENT incomplete behavior as its expected, passing result — a green test
documenting a known gap, not a red one hiding it. The oracle and compiler
also independently agree (non-diverging), so no executed conformance
comparison is silently wrong; the gap was found by deliberately writing a new
discriminating probe, which is what discovery work is for, not by a
mechanism that should have caught it and didn't. Implementing genuine
two-delimiter intersection also requires answering a real semantics question
(exact intersection ordering across TALLYING/REPLACING/CONVERTING, both
single- and multi-item forms, at nine call sites in two engines) rather than
restoring a previously-correct behavior — that is design work, matching rung
5's "new frontend semantics" exactly, not rung 1. **Conclusion: VM-058 is
correctly ranked at rung 5**, though it is more tightly bounded (nine named
call sites, an existing pinned regression to extend) than VM-041 or VM-060b's
genuinely open-ended design gaps, so it would sort ahead of them within that
rung if rung 5 were reached this round.

**Rung 4 vs. rung 5, and where VM-042 actually belongs.** VM-042 was filed as
"pin Brainfuck's intentional BEAM exclusion... for mutable tape operations" —
itself implicitly rung 5/documentation-adjacent (formalizing an existing
design decision). Before implementing it as written, its premise was checked
against source rather than trusted: `brainfuck-iir-compiler/README.md`'s
"Why no BEAM target?" section (written 2026-05-22) argues BEAM's immutable
substrate makes a compiled mutable tape prohibitively expensive (O(N²·M) for
a naive copy-on-write array) and calls the exclusion intentional and
documented. But `iir-to-beam` PR #11343 (2026-08-13) — three weeks *after*
that README section, and three weeks *before* VM-042's own backlog text was
written on 2026-09-05 — added `alloc_bytes`/`store_byte`/`load_byte` backed
by Erlang's `:atomics` module (fixed-size, off-heap, destructive O(1)
get/put), with that PR's own description stating it was added explicitly
"unblocking Brainfuck (which needs a mutable tape)". Nobody had gone back and
actually tried compiling a Brainfuck program through it since — the README
and `LANG-VM-FEATURE-COVERAGE.md` both kept the stale claim, and VM-042 was
filed against that stale claim without re-checking it. That is rung 3
("incorrect roadmap/status documentation that could send work down a dead
path") hiding inside what looked like a rung-5 item: implementing VM-042 as
literally scoped would have written a permanent regression test asserting a
now-false claim.

A real probe (not a re-reading of the README) settled it: a scratch
integration test called `lang_aot::compile_source_to_beam` directly on all
six Brainfuck matrix programs and ran the output under real `erl`. The three
non-input programs (`++++++++[>++++++++<-]>+.` → `"A"`, the nested-loop
`"HA"` program, and the two-loop `"OK"` program) all lowered and executed
correctly, producing byte-identical stdout to every other backend — through
the exact same `store_byte`/`load_byte`/`putchar` lowering every other
byte-tape/array BEAM program on this column already uses; no Brainfuck-
specific BEAM code exists or was needed. The three STDIN programs (`,+.`,
`,.,.`, `,[.,]`) failed lowering with a clean, named `UnsupportedOp: …
call_builtin Some(Var("getchar")) is not in the BEAM builtin set` — `getchar`
was never added when the memory ops landed, because that PR's motivating
workload (the tape loop) never needed it. So the real, narrower shape of
VM-042 is rung 4 (missing backend parity for an already-implemented
feature — three Brainfuck rows just needed their existing, already-working
lowering promoted to a declared matrix cell) plus a correctly-scoped input
refusal pin, not rung 5 design work and not a literal implementation of the
stale premise. This outranks continuing the ~42 undeclared COBOL BEAM rows
(also rung 4, but strictly a repeat of an already-proven pattern with no
documentation-correctness angle) for the same reason VM-061 outranked its
own rung-4/5 alternatives: a stale-documentation finding takes priority once
found, per the policy's own ordering, and this one was hiding directly
inside the very item the queue said to pick up next.

### VM-042 discovery: VM-D031 — the Brainfuck BEAM exclusion was stale, not intentional

Filed as its own discovery entry (see the Discovery log below) rather than
folded silently into the contract: `brainfuck-iir-compiler/README.md` and
`LANG-VM-FEATURE-COVERAGE.md`'s Brainfuck row both asserted, as settled fact,
that BEAM tape mutation was unsupported by design. Three of six matrix rows
already ran correctly against real `erl` by the time this was checked. The
premise was correct for roughly three months (2026-05-22 to 2026-08-13) and
silently wrong for the roughly four weeks after `:atomics` landed, because
promoting a working capability to documentation and to a declared matrix
cell is a step that has to happen deliberately — landing the generic backend
capability does not, by itself, update every frontend's specific claims
about it.

### VM-042 contract

Promote the three non-input Brainfuck matrix rows
(`lang-aot/tests/lang_matrix.rs`) to declare `Beam` in their `backends` list,
based on the real `erl` execution above (42 → 45 declared Brainfuck cells).
Add a dedicated real-`erl` corpus test (`portable_text_stdout_brainfuck_beam_corpus`,
mirroring the existing Oct/Nib/FLOW-MATIC BEAM corpus tests) asserting
exactly those three cells execute with the expected stdout. Add an explicit
frontend-vs-backend split test
(`brainfuck_beam_stdin_rows_refuse_at_backend_not_frontend`) proving all
three STDIN rows compile through `compile_source_to_iir` (the frontend, plus
shared IIR passes) without error, and are refused ONLY by
`compile_source_to_beam` (the BEAM backend specifically), with the refusal
naming `getchar` — the exact "distinguish supported frontend compilation
from backend refusal" VM-042 always asked for, now correctly scoped to the
one thing that is actually unsupported. Pin the same refusal/acceptance
split at the `iir-to-beam` layer directly
(`call_builtin_getchar_rejected_but_putchar_accepted`): `getchar` must be
named in the validator's rejection, and `putchar` — the same `call_builtin`
allowlist check, and the builtin the three newly-promoted rows depend on —
must remain accepted, as the control proving the refusal is targeted, not a
side effect of a broken validation path. Correct
`brainfuck-iir-compiler/README.md`'s "Why no BEAM target?" section and
`LANG-VM-FEATURE-COVERAGE.md`'s Brainfuck row and grand-total prose to state
the real, narrower finding, and update
`feature_coverage_doc_counts_match_programs_source`'s expected Brainfuck
tuple (6, 42) → (6, 45) together with the doc, per that test's own
requirement. No `iir-to-beam` production code changes: the three promoted
rows needed none, and the `getchar` refusal already existed via the generic
`call_builtin` allowlist.

### VM-042 validation

The `iir-to-beam` package suite passed 98 tests (19 unit + 74 integration,
including the new `call_builtin_getchar_rejected_but_putchar_accepted`, + 5
doc) with all-target Clippy on `iir-to-beam` and `lang-aot` (warnings denied)
clean. The three new/updated `lang_matrix.rs` tests
(`feature_coverage_doc_counts_match_programs_source`,
`portable_text_stdout_brainfuck_beam_corpus`,
`brainfuck_beam_stdin_rows_refuse_at_backend_not_frontend`) pass, and all
other BEAM tests (Oct/Nib/FLOW-MATIC/COBOL corpora, arithmetic/comparison/
move immediates, the putchar loop-state proof) pass unchanged alongside
them — twelve BEAM-tagged tests total, no regression. The full
`non_algol_matrix_every_proven_cell_agrees` capstone passed: 210 programs,
**1341** cells exercised (was 1338), 210 skipped (the same host-wide missing
`ilasm` pattern every prior slice reports), zero failures, in 535.91s —
exactly the three newly-promoted Brainfuck cells accounted for, confirming
the doc's corrected grand total (1548 → 1551) against a live run rather than
arithmetic alone.

Reprioritize the remaining ~42 undeclared COBOL BEAM rows against VM-041,
VM-060b and VM-058 after this merges, following the same real policy
comparison run above. VM-058 was re-examined this round and confirmed to
stay at rung 5 (see above) — nothing found here changes that. Twig
records/closures on BEAM and the remaining Twig dynamic-string BEAM rows
(VM-041) still have no scoped backlog entry establishing what a closure or
record even looks like as a BEAM term, so they remain the least-bounded
option until someone does that design work.

## VM-061 feature-coverage recount (selected after #14714 merged)

`git fetch origin && git merge origin/main` confirmed this worktree already
sat at `9d62ab10c1` (PR #14714, "VM-040 COBOL BEAM condition-name/EVALUATE
probe"), merged with all applicable checks green. `gh pr list --state open`
showed no LANG-VM PR in flight — the six open PRs were Punjabi
human-languages, a mosstyle bool-slot cascade, and four dependabot bumps
(npm/GitHub Actions), none of which touch this backlog.

Running the "Prioritization policy" order for real, not by habit: skimming
back through the last several sections (VM-040's four COBOL BEAM slices,
VM-060a, VM-059, VM-039a–d, VM-038, VM-049) found no red executed cell and no
missing-CI-protection gap — every trailing validation paragraph reports
positive sentinels, and VM-024/VM-032 already keep the non-ALGOL matrix and
Windows execution in normal CI, so rungs 1 and 2 are clear again. Rung 3
("incorrect roadmap/status documentation that could send work down a dead
path") is where **VM-061** sits, per its own logging in the prior section:
`LANG-VM-FEATURE-COVERAGE.md`'s per-frontend table was measured stale against
a fresh `non_algol_matrix_every_proven_cell_agrees` run. That structurally
outranks both remaining rung-4/5 candidates the prior section named for
reprioritization — the ~42 undeclared COBOL BEAM rows (rung 4, missing
backend parity for an already-implemented feature) and VM-041/VM-060b (rung
5, new frontend semantics/design, and VM-060b is design work besides, as
every prior deferral of it already found) — by the policy's own stated order,
not by convenience. The only question worth checking before committing to
that pick was whether VM-061 is actually the small, bounded task it looks
like, or secretly a larger design problem in disguise (the prompt driving
this session named exactly that risk); investigating first, before selecting
anything, was the right call here.

Investigation: a brace-balanced parse of every `Prog { lang: Language::X, ..,
backends: &[..] }` literal in `PROGRAMS` (`lang_matrix.rs`, 455 entries)
computed each language's exact row count and total declared-cell count
(`sum(backends.len())` over its rows) directly from source — not a hand count,
not a quick regex, matching the standard the prior section's own note asked
for. Non-ALGOL rows summed to exactly 210 and non-ALGOL cells to exactly
1548, matching the executed `1338 exercised + 210 skipped` total precisely,
confirming the parse itself is trustworthy before trusting any per-row number
it produced. Comparing every non-ALGOL row against the live doc found six of
seven already exactly correct — Nib (26/208), Brainfuck (6/42), Dartmouth
BASIC (51/357), Oct (12/96), FLOW-MATIC (8/60) and COBOL-60 (58/422, already
independently reverified twice by the prior slice) all matched source with no
drift at all. Only Twig was actually wrong: the doc's "343" was `49 rows × 7
standard backends`, silently excluding its 20 Beam-declaring rows' extra
cell each, while Nib/Oct/FLOW-MATIC/COBOL-60's numbers already used the OTHER
convention (every declared backend, Beam included) with no comment anywhere
recording that the two conventions disagreed. The true Twig total is 363.
The prior section's "roughly 66 cells" undercount estimate was itself the
rough guess it warned it might be — the real, source-verified gap is exactly
Twig's missing 20 cells, and correcting only that one number makes the
table's non-ALGOL rows sum to exactly 1548.

This confirms the small-and-bounded reading: VM-061 was one wrong cell count
plus a documentation-convention inconsistency, not a design problem, so it is
correctly ranked and correctly picked over the rung-4/5 alternatives — a real
comparison, not a coin flip dressed as one.

While computing this, ALGOL 60's own doc row (233 rows / 1631 cells) was
observed to no longer match the live corpus (245 rows today) — but ALGOL is
owned by a separate, actively developing campaign per the "Ownership
boundary" section below, and VM-061's own scope (as logged in VM-D030) never
extended a mandate to correct or pin ALGOL's count. That row is left
untouched here; noted for the record, not selected as work.

### VM-061 contract

Fix `LANG-VM-FEATURE-COVERAGE.md`'s Twig row (343 → 363 declared cells;
row count and every other frontend's numbers are unchanged) and correct the
surrounding prose that claimed a ~66-cell, broad-frontend undercount to
instead state the real, narrower finding. Pin the fix against future drift:
add `feature_coverage_doc_counts_match_programs_source` to `lang_matrix.rs`,
asserting the exact (rows, total cells) pair for each of the seven non-ALGOL
frontends that has rows in `PROGRAMS` today (Twig, Nib, Brainfuck, Dartmouth
BASIC, Oct, FLOW-MATIC, COBOL-60) plus a zero-rows assertion for McCarthy
Lisp and Macsyma (both use dedicated capstone files instead, and the doc's
"0/0 means dedicated coverage" reading depends on that staying true). ALGOL
60 is deliberately excluded from the new assertion, matching the doc's own
scope decision above. A future slice that adds or removes a `Prog` for any
asserted language must update both this test's expected tuple and the
matching doc row together, or a normal `cargo test -p lang-aot --test
lang_matrix` run fails — this is the "checked test assertion... so it cannot
drift silently again" VM-061 asked for, not a script run by hand and then
forgotten.

### VM-061 validation

`cargo test -p lang-aot --test lang_matrix feature_coverage_doc_counts_match_programs_source`
passes against the corrected numbers (and was confirmed to actually exercise
the check by first running it against the pre-fix Twig figure of 343, where
it failed with the expected assertion message, before the doc was corrected
to 363). The full `lang_matrix` test binary compiles clean under this change
(a doc-only + test-only change; no production `lang-aot`, frontend or backend
crate was touched). Focused Clippy on `lang-aot` with all targets and
warnings denied is clean. No lowering or runtime code changed, so no backend
regression is possible from this slice; the full non-ALGOL matrix is not
separately rerun beyond the new test's own pass, since nothing it exercises
changed the corpus's actual results, only the accounting of them.

Reprioritize the remaining ~42 undeclared COBOL BEAM rows against VM-041
(Twig dynamic-string isolation), VM-060b (host input design), VM-042
(Brainfuck BEAM-refusal pin) and VM-058 (INSPECT BEFORE/AFTER intersection)
after this merges, following the same real policy comparison run above.

## VM-040 COBOL BEAM condition-name/EVALUATE probe (selected after #14700 merged)

`git fetch origin && git merge origin/main` confirmed this worktree already sat
at `1078ef0305` (PR #14700, "VM-040 COBOL BEAM signed/algebra probe"), merged
`2026-09-09T14:10:30Z`. `gh pr list --state open` showed no LANG-VM PR in
flight (`14713`, `14712`, `14711`, `14710`, `14707`, `14706`, `14705`,
`14703`, `14702`, `14701`, plus dependabot `14699`/`14698`/`14467`/`7821`
are Journal/Mosaic/human-languages/dependency work, not this backlog). This
merge's own trailing paragraph explicitly named no single next item — it
asked for a real reprioritization across three tracks (more COBOL BEAM rows,
Twig dynamic strings/records/closures on BEAM, VM-060b host input) rather
than an automatic continuation, so this section runs that comparison before
selecting.

Applying the "Prioritization policy" order first: skimming every section back
through VM-046c (the oldest region re-read this session) found no red executed
cell, no missing-CI-protection gap, and no stale roadmap claim — every trailing
validation paragraph reports positive sentinels and passing suites, and
VM-024/VM-032 already keep the non-ALGOL matrix and Windows execution in normal
CI. So none of rungs 1–3 outrank the three candidate tracks; the choice is
between rung 4 (missing backend parity for an already-implemented feature) and
rung 5 (new frontend semantics/design). All three candidates are rung-4 shaped
(COBOL, and Twig dynamic strings, are both already implemented on the seven
standard backends — only BEAM parity is missing), except VM-060b, which its
own three prior deferrals (VM-040 Oct, Nib and the FLOW-MATIC output probe
sections above) already describe as needing "a separate reader/string/ABI
design" before any input implementation can even be probed — that is rung-5
design work, not a bounded proof-promotion slice, so it ranks last again here
for the same reason it did each previous time.

Between the remaining two rung-4 tracks: VM-041 ("isolate Twig captured/
reassigned runtime-string lowering from existing source-local string
metadata") is itself scoped as an isolation/design task — its own wording
requires distinguishing two representations before a single proof can even be
written, which is exploratory work with unknown surface area, not a slice with
a pre-existing probe pattern. Twig records/closures on BEAM have no scoped
backlog entry at all yet; a first probe there would need to establish what a
closure or record even looks like as a BEAM term before any proof exists.
Continuing the ~46 undeclared COBOL BEAM rows, by contrast, reuses the exact
`.skip(N).take(4)`-over-`Cobol60`-filter pattern that has now succeeded
three times running (rows 0–3, 4–7, 8–11), with a real dedicated single-cell
probe test already wired (`portable_text_stdout_cobol_beam_signed_and_algebra`
at `.skip(8)`), a real corpus of already-written, already-oracle-tested COBOL
programs to draw from, and a demonstrated repair discipline (VM-D029, the
`str_slice` contract) for whatever the next four rows expose. This is the
narrower, lower-design-risk slice the backlog has consistently preferred over
open-ended design work (the same reasoning that deferred VM-060b twice
already), so COBOL BEAM rows are selected again.

`awk` over `lang_matrix.rs`'s 58 `Cobol60` rows confirmed 12 already declare
`Beam` (the first three probe batches, file-order rows 0–11) and 46 do not,
matching the "~40 undeclared" estimate. Row order after the twelfth (file
lines 5454, 5475, 5495, 5515) is: a level-88 condition-name `IF`, a level-88
multi-value/THRU condition-name (first COBOL BEAM row needing bitwise `and`/
`or` folding), `SET condition-name TO TRUE`, and a symbolic `>=` relational —
all boolean/comparison lowering reusing already-BEAM-proven `cmp_*`/`const`/
branch ops, not the character/EVALUATE/STRING/pointer families the merge
paragraph listed as remaining (those appear later in file order). Run this
next four-row slice (`.skip(12).take(4)`) on real Erlang, matching the
established discipline: probe before declaring, commit a bounded contract for
any newly exposed defect, promote only executed cells.

### VM-040 COBOL BEAM condition-name/EVALUATE validation

All four selected programs pass on real Erlang in the new
`portable_text_stdout_cobol_beam_condition_names_and_evaluate` test, with no
new `iir-to-beam`/`ir-to-beam` defect: each reuses `cmp_*`/`const`/branch
lowering already proven by earlier COBOL BEAM rows, so no production code
changed this slice. All ten BEAM `lang_matrix` tests pass together (12 Oct,
26 Nib, all 16 now-declared COBOL rows across four probe batches, plus the
immediate-arithmetic/comparison/`putchar`-state regressions), 31.51s.
`iir-to-beam`'s package suite is unchanged at 97 tests (19 unit, 73
integration, 5 doc); focused Clippy on `lang-aot` and `iir-to-beam` with all
targets and warnings denied is clean. Sixteen of 58 COBOL rows now declare
BEAM (422 total declared cells). The full `non_algol_matrix_every_proven_cell_agrees`
capstone passed: 210 programs, 1338 cells exercised, 210 skipped (every
program's CLR cell, the same host-wide missing-`ilasm` pattern prior slices
reported), zero failures, in 459.58s.

That full run's own reported total (1338 + 210 = 1548 declared non-ALGOL
cells) is **VM-D030**: `LANG-VM-FEATURE-COVERAGE.md`'s per-frontend table,
last hand-updated for VM-047b/VM-057/VM-047c/VM-039b, undercounts the actual
corpus by 66 cells even after this slice's COBOL correction (58 rows, 422
cells — independently re-verified against source and left unchanged here).
The other frontend rows (Twig, Nib, Brainfuck, BASIC, Oct, FLOW-MATIC) were
not individually re-audited in this bounded slice; a text-based line count
against `lang_matrix.rs` suggests several of their row counts have also
drifted (the corpus grows across many unrelated PRs that do not each revisit
this table), but confirming exact per-row figures needs careful multi-line-
aware parsing, not a quick regex. Per this document's own authority rule
("executed tests… are authoritative until the older roadmap is reconciled"),
the grand-total line now cites the freshly measured 1548 rather than
propagating the stale incremented figure. Fixing every row is out of this
slice's bounded scope (rung 3 of the Prioritization policy — incorrect
status documentation — but a distinct, separately schedulable item from
COBOL BEAM parity); queued as **VM-061**: recompute every
`LANG-VM-FEATURE-COVERAGE.md` frontend row/cell count from `lang_matrix.rs`
source (ideally via a small checked script or test assertion instead of
hand arithmetic, so it cannot drift silently again), reconciling Twig,
Nib, Brainfuck, BASIC, Oct and FLOW-MATIC against the real corpus.

Reprioritize the remaining ~42 undeclared COBOL rows against VM-061,
VM-041 (Twig dynamic-string isolation) and VM-060b (host input design) after
this merges, following the same policy comparison run above.

## VM-040 COBOL BEAM signed/algebra probe (selected after #14684 merged)

The previous top-of-queue entry here ("VM-040 FLOW-MATIC BEAM output probe,
selected after #14646") had already been completed and superseded by the time
this session picked up the file: PR #14665 (`9af6235015`) executed it, and two
further COBOL BEAM slices — PR #14670 (`55f803c1ee`, initial COBOL output) and
PR #14684 (`54cece75db`, COBOL control and rounding) — had already merged past
it without this section being updated. `git log --oneline --all | grep -i
beam` and `gh pr list --state open` confirmed no PR was in flight for this
family; re-deriving the actual next item from the file's own "### VM-040 COBOL
control and rounding validation" paragraph (the true most-recent entry,
referencing #14684) and the live `lang_matrix.rs` corpus (all rows through
COMPUTE precedence already declare `Beam`) selects the next bounded four-row
COBOL slice: signed numeric overpunch, alphanumeric MOVE + comparison,
COMPUTE exponentiation and nested COMPUTE division.

Run all four on real Erlang using the established `.skip(N).take(4)` pattern
over the `Cobol60` filter. Commit a bounded contract for any newly exposed
backend defect before changing it. Promote only executed cells.

### VM-040 COBOL BEAM str_slice contract

The alphanumeric MOVE + comparison probe refused at validation:
`UnsupportedType: … op "str_slice" has type_hint "str"; only the ASCII string
subset is supported`. Every prior BEAM row's DISPLAY formatting used only
`str_const`/`str_concat`/`putchar`; this is the first COBOL BEAM row whose
`MOVE` truncation (`cobol-iir-compiler`'s `move_char_item`) needs a slice.
Add `str_slice` to `iir-to-beam`'s accepted ASCII-string ops, lowering
`[start, end)` to `lists:sublist(List, start+1, end-start)` via `call_ext`
(BEAM strings are character lists; `lists:sublist` is 1-indexed and takes a
count). Register it with the liveness pass that saves variables across a
clobbering call, and stage its three arguments through scratch registers
above `next_reg` before the call, mirroring `store_byte`/`array_set`'s
existing parallel-move-hazard discipline. Then rerun the four selected
programs before declaring BEAM coverage.

### VM-040 COBOL BEAM large-literal sign contract (VM-D029)

With `str_slice` fixed, the nested COMPUTE division probe (`R = A / B + C`,
A=10 B=3 C=2, scale-12 intermediate) passed lowering and validation but
returned the wrong VALUE: `000053` instead of the oracle's `000533` — exactly
a factor of 10 short. Isolating the arithmetic chain in a standalone module
found the actual corruption one step earlier: `const` with a large positive
literal came back NEGATIVE on real `erl` whenever its minimal big-endian
magnitude had its leading byte's high bit set (confirmed by table:
`4_000_000_000` → `-294967296`, `2^31` → `-2147483648`, `2 * 10^12` →
`-99511627776`; `10^10`, `10^11` and `2^32` — whose leading byte's high bit
is clear — round-tripped correctly already). The bug is in the shared
`ir-to-beam` compact-term encoder, not this crate's lowering:
`encode_compact_term`'s "Large form" stripped every leading `0x00` byte
regardless of the operand's `U`/`I` tag, which is correct for `U` (read back
as a plain magnitude) but wrong for `I` (read back as two's complement, where
the leading byte's high bit doubles as the sign).

Fix `value_to_be_bytes` to take a `signed: bool` and, for the `I` path, find
the minimal TWO'S-COMPLEMENT encoding directly — strip a leading `0x00` only
while the next byte's high bit stays clear, and symmetrically strip a leading
`0xFF` only while the next byte's high bit stays set — floored at 2 bytes
(the "Large form" header cannot represent a length below 2, and the naive
signed-minimal rule alone underflows `(length - 2) as u8` for a small
negative literal like `-7`, whose full `u64` bit pattern is enormous and so
reaches this branch purely by sign, not size). Add round-trip regressions
covering the boundary values found above plus `i64::MIN`/`i64::MAX`, then
rerun every existing BEAM row (Oct, Nib, all prior COBOL) to confirm no
regression from the encoding change itself.

### VM-040 COBOL signed/algebra validation

All four selected programs (signed overpunch, alphanumeric MOVE + compare,
COMPUTE exponentiation, nested COMPUTE division) pass on real Erlang and in a
fresh matrix process with positive execution sentinels. Twelve of 58 COBOL
rows now declare BEAM (418 total declared cells; non-ALGOL capstone total
1478). All 12 Oct and 26 Nib BEAM programs, and the eight already-declared
COBOL BEAM rows, pass again after both fixes. `iir-to-beam`'s suite passed 97
tests (19 unit, 73 integration — including two new real-`erl` `str_slice`
tests and one new real-`erl` large-const test — 5 doc), and `ir-to-beam`'s
passed 70 tests (65 unit including new signed-encoding regressions, 5 doc)
with all-target Clippy clean for `ir-to-beam`, `iir-to-beam` and `lang-aot`.
No full seven-standard-column rerun is claimed.
VM-D029 (the encoder sign bug) is filed above rather than as a separately
ranked backlog item because it was fixed within this bounded slice; it is a
general defect and could in principle have affected any prior BEAM row whose
literal happened to land in the same byte-boundary range, but Oct, Nib and
all eight prior COBOL BEAM programs re-passed unchanged, so no other row was
actually affected in practice. Reprioritize the remaining ~40 undeclared
COBOL rows (character/EVALUATE cascades, reference modification, STRING
SIZE/delimiters, pointer/overflow) against Twig dynamic strings/records/
closures and VM-060b host input after this merges.

## VM-040 FLOW-MATIC BEAM output probe (selected after #14646 merged)

PR #14646 merged as `622208db64` after all46checks succeeded or skipped.
Nib's26portable BEAM programs are complete. Select the four FLOW-MATIC
output/control-flow rows next: they can reuse the proven integer-output path
without conflating output with BEAM input/EOF, which remains a separate slice.

Run the existing scalar output, taken EQUAL, false LESS/GREATER/OTHERWISE,
and jump-chain sources on real Erlang. Current rows386-389 must be rechecked
against source after refresh. Missing erl alone may skip; compile/runtime/output
failures must remain hard. Commit a bounded contract for any newly exposed
backend defect before changing it. Promote only executed cells; keep the four
input/EOF rows explicitly undeclared on BEAM until a host-reader proof exists.

## VM-040 Nib BEAM probe (selected after #14632 merged)

PR #14632 merged as `9b04b74cb8` after all 46 checks succeeded or skipped.
Oct's twelve portable BEAM cells are complete. Reprioritize Nib next because
its u8 arithmetic shares the newly proven lowering, while its u4/BCD cases
provide discriminating checks for remaining width and representation gaps.

Probe the existing Nib corpus on real Erlang, verifying row/source identity on
fresh main. Return values must match without process-exit masking; preserve
hard failures after runtime detection. Separate any failing u4, checked
arithmetic, or storage operation into a committed bounded contract before
backend edits. Promote only executed cells. Retain VM-028's broader Intel-4004
fidelity audit and VM-060b's host input design as separate backlog work.

## VM-040 Oct BEAM probe (selected after #14621 merged)

PR #14621 merged as `df39688c62` after all 46 checks succeeded or skipped.
VM-060a's token-table bug is fixed. Reprioritization selects the existing Oct
BEAM gap before VM-060b: Oct already has twelve portable matrix programs,
whereas simulator host input needs a separate reader/string/ABI design.

Probe observable Oct stdout, u8 wrapping, and control flow on real Erlang using
the existing matrix runner. Enumerate row-to-source identities on refreshed
main. Do not promote declarations without successful execution sentinels;
missing erl alone may skip, lowering/runtime/output failures may not. If the
probe exposes a lowering defect, log it and define a bounded implementation
contract before editing the backend. Validate all newly promoted Oct BEAM
cells and relevant backend regression suites, then update coverage/docs.
This does not claim the unimplemented Intel-8008 intrinsic semantics (VM-013).

### VM-040 Oct implementation contract: integer output

The real BEAM probe passed the initial void control-flow row, then refused
`fn main() { out(1, 200); }` at validation: print_i64 is outside the predicate
builtin set. Add print_i64 as an output builtin (no result) using the existing
erlang:display/1 integer output sequence, with its argument at srcs[1].
Preserve the predicate lowering and existing io_out path. Then rerun all twelve
Oct programs before declaring BEAM coverage; record further failures separately.

## VM-060a contract (selected after #14613 merged)

PR #14613 merged as `4462675c52` after 46 completed successful/skipped checks.
VM-059's explicit refusal proof is complete. Prioritize VM-060a before BEAM:
the simulator ignores a call token's table byte, so a MemberRef can dispatch
to an unrelated internal method with the same ordinal. This is an observable
wrong-method execution bug independent of input support.

Accept only MethodDef (`0x06`) call tokens. Reject other table bytes before
popping arguments, saving a frame, or changing the current method/PC. Preserve
the simulator's existing panic-based invalid-bytecode convention with an
explicit unsupported-token diagnostic. Regression uses a real internal method
at the same ordinal: a valid MethodDef must execute it, whereas MemberRef and
other table bytes must refuse without changing execution state. Test zero and
out-of-range MethodDef ordinals using existing invalid-token errors.

Run simulator tests and Clippy plus relevant downstream CIL/McCarthy execution
proofs. This slice does not implement host callbacks. VM-060b retains reader,
string representation and callback ABI design before any input implementation.

The red probe dispatched token 0x00000002 into internal method 1 and consumed
its argument. With table validation, all 10 simulator tests pass, including
five refused table bytes, valid same-row MethodDef execution and invalid rows.
The focused encoded CLR corpus executes all 19 McCarthy programs with no skips.
All-target Clippy for clr-simulator and lang-aot is required before publication.
Cargo's twig-vm dependency tree contains no clr-simulator dependency; this
change does not trigger the twig-vm dependency Miri rule.


## VM-059 contract (selected after #14601 merged)

Main refreshed to `5778c35c3c`. PR #14601 merged as `af727bf81b` after
all 46 checks completed successfully or skipped. VM-039 is complete across
seven standard matrix backends. VM-059 remains first because the encoded CIL
API must clearly distinguish its input refusal from the real CoreCLR proof.

The simulator currently has no host callback registry: its `call` dispatcher
uses the token ordinal as an internal method index without checking the table
byte. The encoded lowerer rejects input builtins through a generic whitelist.
This slice pins a deliberate, actionable compile-time refusal for `input_i64`,
`input_str`, and `input_more`, before any artifact is returned. Public generator
validation and the fallible lowerer must agree; textual `emit_il` must continue
to accept the same input programs. No simulator input support is claimed.

Acceptance: named input refusal tests through both public APIs, a textual
emission control for each builtin, the package test suite and all-target Clippy.
Document the distinction in README and changelog. No simulator runtime change.

Validation: the new public-API regression first failed on the generic whitelist
message, then passed for all three builtins with textual emission controls.
The package suite passed 202 tests (103 unit, 95 integration, four doc tests);
all-target Clippy with warnings denied passed. These are refusal/emission
proofs, not an encoded input execution claim.


Discovered **VM-060**, queued next: design table-aware simulator host-call
resolution before adding input. Pin rejection of unbound MemberRef tokens
rather than aliasing MethodDef ordinals; define explicit per-run shared reader,
EOF/malformed-number behavior, string representation, and isolated callbacks.
Then add bounded encoded input execution proofs. Existing emitted host tokens
also need auditing; their presence alone is not simulator execution support.
Reprioritize VM-060 against BEAM VM-040 after this refusal proof merges.

## VM-039d contract (selected after #14574 merged)

Main is `2cc4f753e6`. Real CLR PR #14574 merged after all applicable final-head
checks passed. Reprioritization selects the last standard-matrix adapter slice:
VM and JIT callbacks for the same four FLOW-MATIC input/EOF programs. VM-059
encoded CIL input is separately queued after this slice; no new defect outranks
finishing the seven-column portable stream proof.

Probe both missing callbacks before implementation. Register non-consuming
input_more on the plain VM, the JIT interpreter fallback and GenericCirJit's
compiled path, sharing each run's existing byte queue with numeric/string
reads. Empty input returns zero; repeated peeks preserve all bytes. Preserve
source/input identity, partial-record zero fill and fields unchanged at EOF.
Add Vm/Jit columns only after execution and verify all 28 source/backend cells
with positive sentinels after re-enumerating current row indices. Add stable
shared-queue peek regression and verify the compiled callback actually runs
rather than crediting interpreter fallback. Run BASIC input regressions and
focused Clippy; relevant VM/JIT tests if their implementation changes. Keep
this slice in lang-aot harness code unless a separately reproduced core defect
requires a committed repair contract. Security review precedes a ready PR.

VM-039d probes on row 443 failed in both VM and JIT with an unregistered
input_more builtin. After registration, all 28 FLOW-MATIC cells (443–446,
seven standard backends) passed in fresh processes with execution sentinels
and zero skips. Ten BASIC input cells on VM/JIT also passed. The shared-queue
regression and direct compiled-callback proof passed; all-target lang-aot
Clippy is clean. No VM/JIT core implementation changes were required and no
full matrix rerun is claimed. VM-059 remains the next separately scoped
encoded-CIL input audit after this PR merges.

## VM-039c CLR contract (selected after #14544 merged)

Main is `a355f64a2a`. JVM PR #14544 merged after every applicable check
passed on its final head. No newly confirmed defect outranks the remaining
CLR input/EOF adapter. Preserve the four existing FLOW-MATIC sources, stdin
and expected output; re-enumerate indices on this main before probing.

First establish real CoreCLR execution and reproduce the missing input_more
lowering. On this Windows host the framework ilasm at
`C:/Windows/Microsoft.NET/Framework64/v4.0.30319/ilasm.exe`, added only to the
process PATH, assembled the existing BASIC numeric-input cell for execution
by real dotnet successfully. No NuGet ILAsm pack was found locally. This
avoids changing tool detection before the adapter behavior is established.

Implement non-consuming input_more through the same Console input reader as
numeric/string reads, with stable repeated EOF. Inspect encoded and textual
CIL paths. Probe numeric EOF independently: current textual ReadLine + Parse
contradicts its zero-at-EOF comment; record the concrete failure and bounded
repair contract before changing parsing. Preserve destination widths and
zero-filled partial records, and propagate I/O failures. Add each CLR matrix
column only after real execution; validate four source cases, stable peeks,
existing BASIC input, backend tests and focused Clippy. Security review
precedes a ready PR. VM/JIT callbacks remain the final VM-039 slice.

CLR probes confirmed: row 441 refuses call_builtin input_more. The independent
BASIC EOF test executes on real CoreCLR and throws ArgumentNullException in
System.Int64.Parse. The bounded repair is now part of this slice: use the
matching Int32/Int64.TryParse(string, out value), with dedicated scratch locals
and a zero result on EOF/malformed/overflow input. Do not catch I/O exceptions.
Implement input_more with Console.In.Peek() > -1, converting the boolean to
the destination width. Textual CLR is the matrix's real runtime path; encoded
CIL currently lacks numeric/string input as well, so its input support remains
explicitly outside this proof rather than claiming simulator parity.

CLR validation: all four FLOW-MATIC cells (441–444) and five BASIC input
cells (370–374) passed in real CoreCLR processes with execution sentinels.
Numeric EOF and direct IIR repeated-peek tests passed, including mixed
string/numeric input, blank/malformed fields, final input without newline,
32-bit overflow-to-zero and a 64-bit value above the 32-bit range. All 201
CIL backend tests including doctests and focused all-target Clippy passed.
No full matrix rerun is claimed. VM/JIT remains next after merge.

Discovered follow-up **VM-059**: encoded CIL's call_builtin dispatcher lacks
input_i64, input_str and input_more. The real-CoreCLR textual path above does
not imply support in clr-simulator. After the common VM/JIT EOF slice, define
and prove simulator host input callbacks, or pin a documented clean refusal
where the artifact contract cannot carry a host reader. This is a separate
surface from the seven-column matrix and does not delay its real CLR proof.

## VM-039c implementation contract (selected after #14511 merged)

Refreshed main is `53795b6fe5`. VM-039b merged after every applicable
current-head check passed, including both gates. No newly confirmed defect
outranks the next adapter slice: JVM and CLR input/EOF for the same four
FLOW-MATIC source/input/output cases. Re-enumerate their indices on this main.

Probe each backend before implementing. For JVM, add `input_more` to the
validator and lower it to `env.BasicRuntime.inputMore()J`. The host must share
one lookahead-capable stream across numeric reads, string reads and peeks;
never consume input on repeated peeks and never convert an I/O failure to EOF.
Preserve destination widths and existing BASIC input regressions.

For CLR, inspect both encoded and textual CIL paths and the actual runner:
no simulator-only result may be reported as real CoreCLR execution. The
existing textual input_i64 arm calls Parse even though its comment promises
zero at EOF; a partial-record probe must establish actual behavior before
repair. If confirmed, record and prioritize the bounded parse/EOF repair
contract before changing it. Preserve short signed-decimal line semantics,
zero-filled partial records and unchanged fields when a read starts at EOF.

Add each column only after its four programs agree in real execution, with
positive row/source checks and honest missing-tool gates. Add stable-peek
regressions and run relevant backend suites, existing BASIC input cells and
Clippy. Split JVM and CLR publication if the CLR repair is independently
substantial; retain one active ready PR and security review before every push.
VM/JIT common matrix callbacks remain the final VM-039 slice.

VM-039c JVM probe: row 440 fails compilation because input_more is absent
from the JVM host-class whitelist. Select the JVM adapter as the first PR;
CLR remains a separate probe/repair slice and is not newly declared here.

VM-039c JVM validation: all four cells (440–443) and five existing BASIC
input cells (369–373) passed in fresh JVM processes with execution sentinels.
The Java host regression proves shared numeric/string input, repeated peeks,
blank/malformed fields, final input without newline, stable EOF and hard I/O
failures. The complete JVM backend suite and focused all-target Clippy passed.
No full matrix rerun is claimed. CLR and VM/JIT remain next after this PR.

## VM-039b implementation contract (selected after #14488 merged)

Refreshed main is `1d1db6507d`. VM-039a merged after all applicable checks
passed, including both final gates. No new defect outranks the next adapter:
WASM input/EOF for the identical four FLOW-MATIC rows 439–442.

Add WASM to those rows without changing source, stdin or expected output.
First execute a discriminating cell and record the current failure. Introduce
`env.__input_more() -> i64` alongside the existing input_i64 import: preserve
feature detection, import ordering, function indices and destination writes.
The matrix host adapter must inspect the same per-program byte queue consumed
by InputI64Func without mutating it. Empty input returns zero; repeated peeks
remain stable before and after consumption. Host import errors must remain
hard failures, never post-detection skips. Document the new host ABI.

Validate all four new WASM cells with positive sentinels, direct repeated-peek
and shared-buffer assertions, the complete iir-to-wasm suite and focused Clippy.
Native/LLVM behavior remains the previously merged proof; JVM/CLR and shared
VM/JIT matrix callbacks remain subsequent slices. Update counts by four cells,
not four programs. Security review precedes publishing a ready PR.

The executed source probe fails the WASM host-import whitelist on input_more.
A newly merged ALGOL row shifts these four rows by one; enumerate the actual
corpus rather than relying on previous indices (443 total rows, 233 ALGOL).
The first attempted row 438 passed an older COBOL program, not the intended
FLOW-MATIC source; only the corrected row 439 establishes this refusal.

VM-039b local execution: all four WASM cells (439–442) passed in fresh
processes with positive sentinels. The direct adapter regression proves
repeated peeks share the same buffer as integer reads, including a final
negative value without newline and stable EOF. The complete iir-to-wasm
suite and focused Clippy (warnings denied) passed. No further lowering defect
was found; the JVM/CLR adapter slice VM-039c is next after merge.

## VM-039a implementation contract (selected after #14471 merged)

Refreshed main is `1ff49866a8`. VM-038 merged with every applicable
current-head check green, including both final CI gates. Reprioritization found
no new lowering defect; portable FLOW-MATIC input/EOF remains next.

Split VM-039 into bounded proofs: (a) native AOT and LLVM using their shared
C input runtime, (b) WASM host import, (c) JVM and CLR runtime adapters,
then (d) the complete seven-backend matrix and common VM/JIT callbacks.
Each slice must preserve the existing frontend READ-ITEM lowering and report
only its executed columns. BEAM remains the separate VM-040 inventory.

The portable stream contains one signed integer field per input line.
`input_more()` returns 1 when another field can be read and 0 at EOF, without
consuming the next field. Repeated peeks must be stable. `input_i64()` retains
its existing permissive parse/EOF behavior; this work does not redesign BASIC
input parsing. At EOF before a READ-ITEM, existing fields remain unchanged.
A partial multi-field record retains the frontend's current behavior: read
available fields and obtain zero for missing trailing fields. Blank or malformed
lines remain present fields parsed as zero. The bounded portable proof uses
short valid decimal lines, including negative values and a final line without
newline; extended encodings and long-line parsing remain separate work.

First add a native/LLVM source probe using the existing canonical finite
READ-ITEM / IF END OF DATA / MOVE / WRITE-ITEM / JUMP loop, plus empty input,
multi-field ordering and repeated EOF reads. Record the unsupported builtin
failure before adding `input_more` runtime mapping. Implement a non-consuming
C stdio peek and the necessary native/LLVM symbol declarations/lowering only.
Execute real processes with piped input and exact stdout assertions, retaining
hard failure after tool detection. Add focused peek stability/EOF validation,
run affected tests and Clippy, and obtain security review before a ready PR.

VM-039a probe reproduced both missing paths before production edits: native
AOT refuses `main`; LLVM explicitly rejects `call_builtin "input_more"` as
outside its whitelist. Prioritize the already scoped shared-runtime mapping
repair before publishing the new matrix cells (rows 438–441).

VM-039a local validation: all eight new cells (438–441 on NativeAot/LLVM)
passed in fresh processes with positive sentinels, including a negative final
field without newline. The production C ABI test passes repeated peeks before
and after consumption and over empty input. The x86_64, AArch64 and LLVM
package suites passed. These four rows declare only NativeAot and LLVM;
WASM is next (VM-039b), not yet claimed by this proof.

## VM-038 implementation contract (selected after #14463 merged)

Refreshed main is `7593f1253d`. VM-057, VM-047c and VM-049 have
already merged; no open LANG implementation PR remains. The next ranked
item is the existing 21-program Macsyma integer/assignment corpus on BEAM.
Compile every source through the public BEAM entry point even without Erlang.
When `erl` is available, execute every emitted module and compare the full
signed integer with the existing oracle. A compilation, process or parse
failure after tool detection must fail, never turn into a skip. Preserve the
existing seven-backend floor and add BEAM to the common agreement test.
Symbolic results remain VM-048. If the probe reveals a lowering defect,
record and prioritize its repair before changing production code. Validate
focused execution, the existing conformance suite and Clippy; inspect hosted
tool availability so local proof is distinguished from hosted coverage.

VM-038 probe: all 21 programs execute on the installed Erlang runtime with
zero skips and correct signed results; no production lowering repair is needed.
Hosted setup-beam is gated by `needs_elixir`, but `lang-aot/BUILD` declares
only dotnet. Add the supported `# needs-toolchain: elixir` declaration so a
LANG-only PR requests Erlang, matching the existing dotnet declaration pattern.
This is a test-toolchain dependency, not a new language runtime dependency.

VM-038 local validation: the expanded conformance suite passed all three
tests, including 21 programs across eight backends (168 agreements), in
56.53 seconds. BEAM, CLR simulator, JIT, JVM, LLVM, VM, WASM and native AOT
all executed; zero backend skips. The separate BEAM proof passed all 21
programs. Focused Clippy passed with warnings denied. No new lowering defect
was discovered; VM-039 portable FLOW-MATIC input/EOF remains next after merge.

## VM-049 implementation contract (selected after #14449 merged)

Refreshed main is `df33215f43` (VM-047c merged as `db77422ad1`). No new
executed failure or lowering defect was found; the ranked queue's next item
was VM-049: a real .NET lane for the existing Macsyma arithmetic corpus.

Investigation before writing any test found the actual gap narrower than "add
CLR support to Macsyma" — Macsyma already has full CLR coverage through
`macsyma_conformance.rs`'s `run_clr` backend, which runs the emitted CIL on
the **in-repo** `clr-simulator`. What was missing is a **real** CoreCLR proof
of the same corpus, mirroring the `CLR-real` chapter McCarthy's `conformance.rs`
and five `clr_real_*.rs` files already established via
`compile_source_to_cil_text` → real `ilasm` → real `dotnet`. That harness
(`tests/clr_support/mod.rs`) was McCarthy-only: `run_on_real_clr` hardcoded
`Language::McCarthyLisp`. Generalized it to `run_lang_on_real_clr(language,
src, tag)`, with `run_on_real_clr` kept as a thin McCarthy-only wrapper so the
five existing McCarthy files need no changes.

Added `tests/clr_real_macsyma.rs` over the identical 21-program corpus
`macsyma_conformance.rs::PROGRAMS` already agrees on across the simulator
floor (literals, all four binary ops, precedence/chains, exact division,
unary, assignment/reference, multi-statement chains) — not a narrower
hand-picked subset. Before assuming `iir-to-cil-bytecode::emit_il` needed new
op support (McCarthy's doc comments describe it growing per-slice: cons,
predicates, `COND`, symbols, lambda — arithmetic was never listed), traced
Macsyma's actual lowered shape: the frontend always emits `call_builtin
"+"/"-"/"*"/"/ "` (per `macsyma_conformance.rs`'s own module doc), which the
shared `iir-builtin-lowering::dynamic_arith` pass unconditionally expands to
`unbox`/`add`/`box` **before any backend sees it** — ops `emit_il` already
lowers for McCarthy's cons/predicate paths. A toolchain-independent
`macsyma_emits_valid_cil_text_for_full_corpus` test (compiles all 21 programs
to `.il` text, asserts success, no `dotnet`/`ilasm` needed) confirmed this
locally: it passed on the first attempt, so no `emit_il` change was made. This
is a coverage-lane addition on already-implemented lowering, not new codegen —
the same shape VM-021 found for Macsyma's original CLR-simulator wiring.

Investigating why McCarthy's pre-existing `clr_real_*` lane always reported a
skip rather than sometimes a real pass — even on this sandbox's known missing
`ilasm` (VM-047c), hosted CI installs it, so *some* prior hosted run should
have exercised it — surfaced **VM-D028**: `lang-aot/BUILD` never declared
`# needs-toolchain: dotnet`. `lang-aot`'s own bucket language is "rust", and
`.github/workflows/ci.yml` only runs `actions/setup-dotnet` and restores the
`ilasm` NuGet package when the CI-wide `needs_dotnet` flag is set — which
without this declaration, no PR touching only `lang-aot` ever set. Confirmed
against #14449 (VM-047c)'s own hosted Linux job log: `needs_dotnet=false`, and
the `Verify tools` step's `.NET: $(dotnet --version)` line was generated but
its guard was `if [ "false" = "true" ]`, so it never executed — meaning
McCarthy's CLR-real column has *never* actually run on a normal PR merge-gate
CI, only on a forced main-branch full build (which sets every toolchain flag
via `is_main`). Every CLR-real test still skipped *correctly* the whole time
— the tool gate itself was never wrong — but "correct skip" and "hosted CI
proves real execution" are different claims, and the backlog's own acceptance
criteria require the latter. Fixed by adding the declaration, the exact
pattern `java-to-semantic-ir/BUILD` already uses for its own extra Python
dependency; also added `clr_real_macsyma` to `BUILD`'s protected target list.
This is squarely in VM-049's scope: without it, the new Macsyma real-CLR lane
would have inherited the identical silent-skip gap on its own PR.

VM-049 local execution: `macsyma_emits_valid_cil_text_for_full_corpus` passes
all 21 programs (no toolchain needed). `macsyma_runs_on_real_coreclr` reports
the expected honest skip on this host (`dotnet`/`ilasm` both absent,
consistent with VM-047c). `macsyma_conformance.rs`'s simulator-floor test is
unchanged and still passes 21 programs × 7 backends (147 agreements);
`conformance.rs`'s McCarthy W16 capstone still passes 19 programs × 8 backends
(152 agreements) and its own five `clr_real_*.rs` files still report the same
honest skip after the shared-helper generalization — no regression. Focused
Clippy on `lang-aot` with all targets and warnings denied is clean. The go
build-tool's toolchain-declaration unit and fixture tests
(`internal/discovery`, `toolchain_declaration_fixture_test.go`) pass unchanged,
confirming `# needs-toolchain: dotnet` parses the same way
`java-to-semantic-ir/BUILD`'s existing directive does. The full non-ALGOL
matrix (`non_algol_matrix_every_proven_cell_agrees`) passed 206 programs, 1256
cells exercised and 206 skipped (every program's CLR cell, the same
host-wide missing-`ilasm` pattern VM-047c reported), zero failures, in 530.34
seconds — identical shape to the pre-VM-049 baseline, as expected since this
item adds a dedicated-suite lane, not unified-matrix rows. The two other
`BUILD`-protected `lang_matrix` targets (`t7_differential_random_basic_`,
`portable_text_stdout_`) also pass.

Hosted CI is the first environment that can actually prove or disprove real
CoreCLR execution for both the new Macsyma lane and McCarthy's pre-existing
one, now that `needs_dotnet` will actually be set for this PR. If hosted CI
also cannot produce a real pass (rather than a skip) for `clr_real_macsyma`
and the existing `clr_real_*` files, that is a further, separate finding to
record — not something to paper over by reverting the `BUILD` declaration.

## VM-056 repair contract (discovered during VM-047b)

All five replacement programs fail on WASM (one bounds trap, four corrupted
outputs), while the other 30 cells and 292 oracle tests pass. INSPECT emits
`str_concat result = result, character`. The WASM runtime concat writes its
new allocation handle into the destination before reading source lengths and
bytes. When that destination aliases an operand, subsequent reads use the new,
uninitialized block rather than the original string.

Prioritize VM-056 before publishing VM-047b. Keep both operand locals intact
until the new header and bytes are written; defer destination assignment until
all reads and bump accounting finish. Preserve memory capacity checks and the
literal fast path. Add direct executable regressions for destination aliasing
the left, right and both operands. Re-run all 35 replacement cells, the full
WASM package suite, focused Clippy and the complete non-ALGOL matrix.

VM-056 local repair: the direct left/right/double-alias regression passes after
reproducing wrong output before the fix. All 35 replacement cells now pass in
fresh processes with positive sentinels and zero skips. All 236 WASM package
tests (including doctests), 292 INSPECT oracle tests and focused Clippy pass.
Full non-ALGOL matrix validation passed: 200 programs, 1420 exercised cells,
zero skips in 635.72 seconds.

Discovered follow-up **VM-057**: inspection found that runtime `str_slice`
also assigns its destination before reading the source during the byte copy.
An aliased source/destination may be overwritten similarly. Add a discriminating
runtime-parameter probe before changing this path; rank this suspected shared
lowering defect ahead of VM-047c when the current PR merges. It is not yet an
executed failure and is outside the concat repair's validated scope.

## VM-057 implementation contract (selected after #14407 merged)

A discriminating IIR-level probe confirmed the suspected hazard before any
fix: a `str_slice` whose destination register aliases its source (`reg_map`
keys one wasm local per variable *name*, so `str_slice s = s, start, end`
reads and writes the identical local) produced wrong bytes, not merely a
theoretical risk. The runtime path wrote `rd = i32.wrap(bump)` — the fresh
block's handle — before the header write and the `memory.copy` that reads
`src_base` back out of that same local, so an aliased read observed the new,
uninitialized block instead of the original string.

Repaired with VM-056's exact pattern: address the fresh block through the
not-yet-advanced `bump` global for both the header store and the
`memory.copy` source/destination math, keeping every source-local read
(`push_src_len`, `push_src_base`, and the shared `push_start`/`push_end`
bounds/index reads) intact until after the last one completes; only then
push the bump-based handle, advance `bump`, and `local.set` the destination.
Bounds checks, the `$__ensure_capacity` growth call and the literal fast path
are unchanged.

Traced the actual COBOL-reachable trigger rather than assuming the reference-
modification shape the discovery note suggested: `ref_mod_slice` (COBOL
`base(start:len)`) always materializes its slice into a fresh temporary
before the final reshape write, so `MOVE`/comparison reference modification
never reaches this hazard. The real single-instruction alias is
`STRING <item> DELIMITED BY SIZE INTO <same item>` — `string_source` returns
a `Name` operand's live register directly, and a lone sending field skips the
`str_concat` combining loop entirely, so the truncating `str_slice`'s
destination and source are the identical register.

VM-057 local repair: the destination-aliases-source IIR probe reproduces
corrupted output before the fix and passes after it, alongside the existing
left/right/both-alias `str_concat` regressions in the same file. A COBOL
`STRING S DELIMITED BY SIZE INTO S` program (`S PIC X(5) VALUE "ABCDE"`)
gets a seven-backend `lang_matrix` cell expecting `S` unchanged, plus an
oracle-agreement sanity check on the generic JIT/interpreter path (which was
never exposed to this WASM-only defect). All 237 `iir-to-wasm` package tests
(including doctests) and all 632 `cobol-iir-compiler` tests (including the
new construct's oracle proof) pass; focused Clippy on `iir-to-wasm`,
`cobol-iir-compiler` and `lang-aot` (tests included) is clean. Audited every
other `iir-to-wasm` lowering site that shares `str_concat`/`str_slice`'s
bump-allocate-then-`memory.copy` shape: `alloc_array` copies no existing
operand's bytes (only a requested length), so it was never exposed, and no
third such site exists. Spot-checked `iir-to-llvm`'s `str_slice`/`str_concat`
lowering, which already reads every operand into a value before overwriting
the destination `env` entry (with a comment recording that ordering is
deliberate) — this bug's shape is specific to a backend that reuses one
mutable local per variable name, not a defect to assume recurs in every
backend.

No further executed failure or lowering defect was found. VM-047c (INSPECT
BEFORE/AFTER regions and absent-delimiter asymmetry) is next per the existing
ranked queue.

## VM-047c implementation contract (selected after #14426 merged)

Refreshed main is `9aa15c97e9` (VM-057 merged as `5206f8ce2a`). No further
executed failure or lowering defect was found; INSPECT BEFORE/AFTER region
proofs are next per the existing ranked queue.

Investigation before writing any matrix row found the region window itself
already fully implemented and already exhaustively tested against the
`cobol-runtime` oracle: `emit_inspect_region_window` (compiler) and
`region_window` (oracle) both derive a single `{BEFORE|AFTER} x` window with
the documented ISO not-found asymmetry — `BEFORE` with an absent delimiter is
the WHOLE source, `AFTER` with an absent delimiter is an EMPTY region — across
TALLYING FOR ALL/LEADING/CHARACTERS, REPLACING ALL/LEADING/CHARACTERS,
CONVERTING, their multi-item forms, and the combined TALLYING…REPLACING
statement with an independent region per half. This item's actual bounded
proof is therefore promoting that already-implemented, already-tested
behavior to the seven-backend `lang_matrix` columns, not new frontend design.

Add five ASCII canonical programs on all seven standard backends: TALLYING
FOR ALL narrowed BEFORE a present delimiter, then BEFORE an absent one
(whole-source asymmetry); TALLYING FOR ALL narrowed AFTER a present
delimiter, then AFTER an absent one (empty-region asymmetry); REPLACING ALL
narrowed BEFORE a present/absent delimiter with bracket markers showing the
untouched tail; REPLACING ALL narrowed AFTER a present/absent delimiter,
where the absent case leaves the bracketed source byte-identical to the
original; and BEFORE used on a TALLYING half together with AFTER on a
REPLACING half in one combined `INSPECT` statement, proving the ISO
tally-then-replace ordering over the identical original bytes.

Investigating the fourth item — "a combined case if the language allows
BEFORE and AFTER together in one clause" — surfaced a genuine, previously
unexercised gap, logged as **VM-D027**: `cobol.grammar`'s `inspect_region` is
wrapped in a `{ }` repetition, so a single delimiter phrase carrying BOTH
`BEFORE x` and `AFTER y` (e.g. `FOR ALL "0" BEFORE "X" AFTER "X"`) parses into
two sibling `inspect_region` nodes, but every reader — the oracle's
`program.rs` and all seven of the compiler's `child_node(_, "inspect_region")`
call sites in `lib.rs` — takes only the FIRST node via `child_node`, silently
discarding a second one instead of computing the ISO-intended two-delimiter
intersection or rejecting the phrase as a later rung. A discriminating probe
(`FOR ALL "0" BEFORE "X" AFTER "X"` over `"00X00"`) confirmed both engines
agree on the current (incomplete) behavior — bare `BEFORE "X"`, count 2 — so
this is a shared, non-diverging limitation, not a compiler-vs-oracle
conformance failure; it does not block or reprioritize this slice. Real
two-clause intersection support touches nine region-parsing call sites across
both the oracle and the compiler (every TALLYING/REPLACING/CONVERTING single-
and multi-item form), which is its own bounded slice, ranked separately below
as VM-058. A regression test pins the current first-region-only behavior so
it cannot silently change shape before VM-058 lands. The COMBINED case this
item actually promotes to the matrix — `BEFORE` on one phrase and `AFTER` on
a different phrase within the same `INSPECT` statement, each independently
regioned — is the standard's other, already-fully-implemented "BEFORE and
AFTER together" reading, and is not affected by the VM-D027 gap.

VM-047c local execution: all 30 available cells (rows 433–437 × NativeAot,
LLVM, WASM, JVM, VM, JIT) pass in fresh processes via the single-cell
`matrix_every_proven_cell_agrees --exact` re-verification path with positive
ran-cell sentinels and zero failures; CLR is an explicit missing-tool skip on
this host (`ilasm` not locatable in the NuGet runtime-pack cache), reproduced
identically on the pre-existing row 432, so it is not a regression. All 649
`cobol-iir-compiler` package tests (83 unit + 22 backend-compat + 638 JIT/
oracle, six of them new for VM-047c and VM-D027, plus zero doctests) pass.
Focused Clippy on `cobol-iir-compiler` and `lang-aot` with all targets and
warnings denied is clean. The full non-ALGOL matrix (`non_algol_matrix_every_
proven_cell_agrees`) passed 206 programs, 1256 cells exercised and 206
skipped — every program's CLR cell, consistent with the host-wide missing
`ilasm` tool rather than any new gap — with zero failures, in 501.12 seconds.

## VM-047b implementation contract (selected after #14400 merged)

Refreshed main is `2755b36eb5`. PR #14400 merged after all current-head checks
passed. No new runtime defect was discovered in tallying; replacement is the
next bounded proof in the existing priority queue.

Add five ASCII canonical replacement programs on the seven standard backends:
ALL using data-name operands and absent matches; LEADING stops at the first
mismatch while ALL reaches later matches; CHARACTERS replaces padded spaces;
a multi-item clause never feeds produced bytes into later items; overlapping
searches use the first written item. Observe repeated source writes and use
bracket markers where spaces matter. Add identical complete-source oracle
comparisons. Run each new cell in a fresh process with a positive sentinel,
all INSPECT oracle tests, focused Clippy, inventory counts and link checks.
Keep region boundaries in VM-047c. Any executed failure takes priority and
requires a committed repair contract before production changes.

## VM-047a implementation contract (selected after #14394 merged)

Refreshed main is `bf906aae9e`. PR #14394 completed VM-046c with all
current-head CI checks green; all three STRING/UNSTRING slices are complete.
No new executed failure outranks the existing coverage queue. Split VM-047:

| Order | Item | Bounded proof |
|---|---|---|
| done #14400 | VM-047a | ASCII INSPECT TALLYING ALL, CHARACTERS and LEADING on all seven standard backends. |
| selected | VM-047b | INSPECT replacement, including first-match and non-rechaining behavior. |
| then | VM-047c | INSPECT BEFORE/AFTER regions and absent-delimiter asymmetry. |

Add three canonical programs: ALL adds to a nonzero counter, a zero-match
inspection leaves it unchanged, and an item delimiter is observed; CHARACTERS
counts the full padded field width and accumulates; LEADING distinguishes an
initial run from later matches, handles no initial match and a fully matching
field. Use ASCII source and existing frontend/oracle semantics. Add identical
source/output oracle regressions for these combined observations. Execute each
new matrix cell in a fresh process with its single-cell sentinel, run the
INSPECT oracle suite and focused Clippy, and update the coverage inventory,
README and changelog. Any observed lowering defect gets a separate committed
repair contract before a fix. Replacement and region proofs remain later slices.

VM-047a local execution: rows 424–426 passed all 21 standard-backend cells
in fresh processes with positive single-cell sentinels and no skips. All 287
INSPECT JIT/oracle tests passed, including the three identical matrix sources.
Focused Clippy passed with warnings denied. No lowering repair was needed.
The corpus now contains 427 rows, including 47 COBOL rows; the non-ALGOL
capstone declares 195 programs and 1385 cells including 20 Twig BEAM cells.

## Prioritization policy

Re-rank before selecting every work item, and whenever current work discovers a
new gap. Apply this order:

1. Red executed conformance cells or silent backend skips.
2. Missing CI protection for already-working conformance.
3. Incorrect roadmap/status documentation that could send work down a dead path.
4. Missing backend parity for an already-implemented language feature.
5. New frontend semantics and historical-machine fidelity.

Each item must add or strengthen an executed cross-backend proof. One item uses
one fresh worktree and one PR; remove the worktree after merge.

## Completion loop and acceptance criteria

The loop keeps one active implementation PR. Before selecting another item,
fetch `origin/main`, inspect the previous PR's actual merge state, record new
discoveries, and rerank this backlog. Fix CI failures and merge conflicts on the
active PR. Publish ready-for-review PRs, never drafts, so draft-specific check
filters cannot suppress validation. Enable auto-merge only after its current-head checks finish green and
no conflicts or blocking reviews remain; do not bypass checks. After merge,
record completion and repeat from a clean feature worktree.

Completion means the scoped language features have executable conformance on
every applicable backend, those proofs run in normal CI, historical-machine
semantics are explicit and tested, and the status documents agree with the
code. A green example is not full language completion. The separate ALGOL
campaign retains ownership of ALGOL semantics. The platform vision's broader
tooling, transpilation, GC, and performance aspirations need their own audited
acceptance inventory (VM-029), rather than an implicit claim of completion.

### 2026-09-05 prioritization

The initial audit selected a directly observed CI gap: `lang-aot/BUILD` runs only the BASIC random differential
tests from `lang_matrix`, so the new non-ALGOL conformance rows are not protected.
The full matrix remains excluded for separately owned ALGOL Linux native-AOT
array failures. VM-024 will run every non-ALGOL `PROGRAMS` row and its declared
backends through the existing strict runner, preserving the full matrix and its
diagnostic single-cell interface. Missing tools remain explicit skips; compiler,
linker, runtime, and result failures remain failures. Report executed and skipped
cells and reject an empty selection. Validate on the local host and all PR CI
hosts before enabling auto-merge. Promote any discovered runtime failure ahead
of further coverage or semantic work.

Executing that selection exposed VM-030 on the very first Windows native cell.
PR #14265 repairs it; 62 library tests and all-target Clippy pass. Both LLVM
`lld-link` and Microsoft `link.exe` run the Windows smoke suite successfully
(eight reported tests, six exercised tests and two explicit GC early returns).
The original failing LANG cell `0:NativeAot` passes with the single-cell sentinel.
Current-head CI remains the merge gate. The subsequent matrix run reproduced
VM-033 (Windows text-output newlines), which now precedes coverage work. After
that repair, missing Windows runtime CI protection (VM-032) precedes the broader non-ALGOL matrix gate:
otherwise this exact regression can recur despite a green Windows job.

## VM-046c implementation contract (selected after #14387 merged)

PR #14387 merged as `8e52198a5b` after all checks passed at `3a0e0e870e`.
VM-046b and discovered VM-054/055 are complete. Full local non-ALGOL regression
passed 184 programs / 1308 cells with zero skips. Reprioritization selects the
remaining pointer/overflow slice; ALGOL stays separately owned.

Add canonical STRING and UNSTRING programs declaring all seven standard
backends. Observe receiver content, pointer writeback and distinct ON OVERFLOW /
NOT ON OVERFLOW markers together. Cover in-range offsets, exact fit, partial
transfer, invalid starting pointers, exhausted input and trailing-delimiter
boundary behavior using the existing frontend oracle contracts. Preserve
untouched receiver bytes with nonblank initializers and bracket output so
padding remains visible. Execute all new cells in fresh processes with ran-cell
sentinels; run frontend oracle tests and focused lint. Normal non-ALGOL BUILD
includes the rows automatically. Log and prioritize any new defect before
claiming parity, with repair specs committed before implementation. Update
inventory counts, README, changelog and validation evidence.

VM-046c local execution: all eight rows (416–423) pass all seven standard
backends in fresh processes with ran-cell sentinels: 56 executions, zero skips.
All 121 frontend STRING/UNSTRING oracle tests and focused matrix Clippy pass.
No runtime repair was needed. The inventory now declares 192 non-ALGOL
programs / 1364 cells; hosted normal BUILD remains the merge gate.

## VM-046b implementation contract (selected after #14377 merged)

PR #14377 merged as `b071b0493c` with all checks green at `3a68a24c33`.
VM-046a and discovered VM-053 are complete. Its full local non-ALGOL matrix
passed 179 programs / 1273 cells with zero skips. Reprioritization finds no
remaining observed red cell ahead of VM-046b; ALGOL stays separately owned.

Add bounded ASCII STRING delimiter and UNSTRING splitting programs on all
seven standard columns. STRING must stop each sender at its first delimiter,
including absent/leading delimiters and an item delimiter, while preserving
the receiver tail. UNSTRING must fit fields to receiver widths, space-fill
empty fields, drop excess fields and leave unused receivers unchanged after
source exhaustion. Use visible markers around receivers to retain padding and
empty-field evidence. Cover literal and item delimiters with existing oracle
semantics; pointer and overflow branches remain VM-046c. Execute each new cell
in a fresh process with a ran-cell sentinel, run frontend oracle checks and
focused lint, and update source-linked inventory counts. Normal non-ALGOL BUILD
must include every row. Log and prioritize any actual backend defect before
claiming these cells proven; commit repair contracts before implementation.

### VM-054/055 discovery and repair contracts (block VM-046b)

The 35-cell survey passes all 25 WASM/JVM/CLR/VM/JIT cells, but every native
and LLVM delimiter cell fails. LLVM refuses nonconstant `str_index` loop
indices (VM-055). Native produces empty output (VM-054): its string-lowering
integer facts fold a loop-carried scan index to its initial constant instead
of reading the current value. The frontend oracle selection passes 121 tests.

Prioritize both failures within this slice. Native string folding must exclude
multiply defined integer registers and invalidate overwritten integer facts;
nonfoldable str_index operations must use the existing bounds-checked runtime
helper, registered for both native ABIs. LLVM must route runtime sources or
indices to that same helper, retaining literal fast paths and correct scalar
result facts. Link the production helper in the matrix. Add focused regressions
for loop-carried indices and runtime sources, execute the new cells, and rerun
the full non-ALGOL matrix. Do not remove backend declarations or change COBOL
semantics to hide failures.

VM-046b/054/055 local validation: all five new rows (411–415) execute on
all seven standard backends with fresh-process ran-cell sentinels: 35 cells,
zero skips. Frontend STRING/UNSTRING oracle selection passes 121 tests;
220 native library tests and 128 LLVM integration tests pass. Focused matrix
and all-target affected-backend Clippy pass. Full non-ALGOL regression is
running; hosted CI remains the merge gate. The inventory declares 184
non-ALGOL programs / 1308 cells.

## VM-046a implementation contract (selected after #14370 merged)

PR #14370 merged as `8504430394` after all current-head checks passed on
`b878adec71`. VM-045 and the discovered VM-050/051/052 repairs are complete.
The full local non-ALGOL matrix passed 176 programs / 1252 cells, zero skips.
No red cell supersedes the next coverage item. Split VM-046 into small proofs:

| Priority | Slice | Acceptance |
|---|---|---|
| done #14377 | VM-046a | STRING DELIMITED BY SIZE: full source widths/spaces, mixed literal/item input, receiver truncation and preservation of a nonblank untouched tail. |
| done #14387 | VM-046b | STRING delimiters and UNSTRING basic splitting: empty fields, receiver fitting and untouched receivers after source exhaustion. |
| done #14394 | VM-046c | STRING/UNSTRING pointer updates and overflow branches with distinguishable outputs. |

VM-046a adds canonical ASCII programs declaring all seven standard backends.
Use visible trailing markers so output normalization cannot hide spaces, and
nonblank initial receiver tails to distinguish STRING from MOVE space-filling.
Run each new cell in a fresh process with a ran-cell sentinel; validate borrowed
semantics against the frontend oracle cases. Normal non-ALGOL BUILD includes
new rows automatically. Preserve frontend/runtime semantics; record and repair
any newly observed defect before claiming parity. Update inventory counts,
README, changelog and backlog evidence. ALGOL remains separately owned.

### VM-053 discovery and repair contract (blocks VM-046a)

New row 410 executes repeated STRING with a changed source. Native/LLVM print
`ABZZZZ|` then `CDZZZZ|`; WASM incorrectly prints the later value twice.
Rows 408/409 pass all seven backends, and 121 frontend STRING/UNSTRING oracle
tests pass. Reprioritize this observed stale string-fact defect before coverage.

WASM's function-wide literal table must not substitute a later assignment for
an earlier read, even when both assignments occur in one basic block. Mark
multiply written string variables as runtime-valued and propagate that status
to their consumers; preserve the single-definition literal fast path. Add a
focused regression proving reassigned literals are runtime values, rerun all
new cells and the full non-ALGOL matrix, and report actual execution/skips.
Keep COBOL semantics and the other backends unchanged.

VM-046a/053 local validation: rows 408–410 each pass all seven standard
backends in fresh processes with ran-cell sentinels (21 executions, zero skips).
The frontend STRING/UNSTRING oracle selection passes 121 tests. The complete
WASM package suite passes 235 tests including doctests. Focused matrix and
all-target WASM Clippy pass. The broader non-ALGOL matrix is running; hosted
CI remains the merge gate. Inventory now declares 179 non-ALGOL rows / 1273 cells.

## VM-045 implementation contract (selected after #14363 merged)

PR #14363 merged as `7291d2684e` after all current-head checks passed on
`afe88dca63`. VM-044 is complete. Reprioritization selects the next existing
coverage gap, VM-045; ALGOL remains separately owned.

Add canonical COBOL ASCII reference-modification programs for literal and
computed start/length, omitted length, IF/EVALUATE comparisons and MOVE into
wider/narrower alphanumeric receivers. Include trailing markers to preserve
padding evidence. Add runtime-invalid start and end cases expecting traps,
matching the existing JIT/oracle bounds contract. Declare all seven standard
backends, execute each new cell in a fresh process with a ran-cell sentinel,
and validate the borrowed semantics against the frontend oracle suite. Normal
non-ALGOL BUILD must include these rows automatically. Do not broaden existing
byte/character or receiver-category semantics. If execution exposes a backend
defect, record and prioritize a bounded repair before claiming parity. Update
inventory counts, README, changelog and validation evidence.

### VM-050 discovery and repair contract (blocks VM-045)

Executing VM-045 reproduced `BackendRefused { function: "main" }` on native
AOT row 402 with computed substring indices. Literal row 401 passed all seven
backends; the frontend reference-modification oracle suite passed 66 tests.
Native string lowering only folds constant slices and leaves runtime slices
for a backend that refuses them, despite the existing bounds-checked
`__twig_str_slice` runtime helper. Prioritize this observed failure within the
current VM-045 PR before declaring its new cells proven.

Route well-formed slices that cannot be folded through `call_builtin str_slice`
and invalidate stale destination string facts. Preserve constant folding and
its invalid-bound traps. Verify runtime output and invalid-bound failures via
the canonical rows, plus focused lowering checks for runtime source/bounds and
reassigned destinations. Use existing runtime/ABI helper plumbing; investigate
any further backend refusal instead of excluding its cells.

### VM-051/052 discovery and repair contracts (block VM-045)

The complete 49-cell survey after VM-050 passed 46 cells. LLVM rows 402 and
405 reject computed `str_slice` bounds as nonconstant (VM-051). WASM row 405
prints `|` and two spaces plus `|` instead of `BC   |` / `BC|` (VM-052).
All seven native cells pass, including both expected traps. Prioritize these
observed failures within this PR before declaring the new rows protected.

VM-051: use LLVM's existing length-prefixed runtime string representation and
shared bounds-checked slice helper for unknown sources/bounds. Preserve its
literal folding, trap behavior and stale-fact invalidation. VM-052: diagnose
runtime MOVE's incorrect WASM output and fix the narrow string-lowering fact
or copy defect responsible, with the failing canonical program as regression
proof. Do not alter COBOL semantics or remove failing backend declarations.

VM-045/050/051/052 local validation: all seven new rows (401–407) pass all
seven standard backends in fresh processes with ran-cell sentinels: 49
executions, zero skips. The frontend oracle suite passes 66 tests; affected
backend libraries pass 281 tests, and LLVM integration passes 127 tests.
Native and LLVM now route computed slices through the production checked
runtime helper. WASM propagates runtime representation from computed bounds
through downstream string copies. Hosted CI remains the merge gate.

## VM-044 implementation contract (selected after #14358 merged)

PR #14358 merged as `669d5fcc1e` after all current-head checks passed on
`a9f733229e`. VM-037 is complete; no new red cell supersedes the existing
coverage queue. Select VM-044, leaving ALGOL with its separate owner.

Add canonical Oct programs that expose while-loop iterations and a returned
function value through stdout, including u8 wrapping across iterations. Add
loop/conditional-break and nested-break programs whose output distinguishes
inner from outer targets. Declare all seven standard backends and run every
new cell in a fresh process with an execution sentinel, reporting absent tools
honestly. Normal non-ALGOL BUILD must automatically cover the new rows.
Preserve frontend/runtime semantics; if a new program exposes a real defect,
log it and prioritize a bounded fix before promoting the affected cell.
Update inventory counts, changelog and backlog evidence. Byte wrapping and
calls must be observed at runtime, not merely validator acceptance.

VM-044 local execution: rows 373, 374 and 375 passed on all seven standard
backends, each in a fresh process with the ran-cell sentinel (21 executions,
zero skips). No runtime fix was needed. The corpus now declares 12 Oct rows
and 169 non-ALGOL programs / 1203 cells. Hosted CI remains the merge gate.

## VM-037 implementation contract (selected after #14349 merged)

PR #14349 merged as `839e4191e8` after all applicable checks passed on
`983ab9fbfe`. Hosted Windows explicitly reported 19 McCarthy native programs
executed; Linux/macOS BUILD also passed. VM-036 is complete. Reprioritization
selects VM-037, the next coverage-only item; ALGOL remains separately owned.

Add canonical FLOW-MATIC programs proving a taken EQUAL branch, untaken
LESS/GREATER branches reaching OTHERWISE, and an unconditional jump chain.
All paths must terminate and wrong paths must print distinguishable output;
use differing record widths/line counts rather than infinite-loop failure
sentinels. Fields start at zero, so this slice does not claim positive LESS or
GREATER comparisons over nonzero input (the EOF-aware input work is VM-039).
Declare all seven standard backends and execute each new cell with strict
existing result assertions, reporting missing tools honestly. Normal non-ALGOL
BUILD must include the new rows automatically. Preserve frontend/runtime
semantics. Update source-linked inventory counts and backlog validation.

VM-037 local execution: new canonical rows 374, 375 and 376 each passed
NativeAOT, LLVM, WASM, JVM, CLR, VM and JIT in fresh single-cell processes.
All 21 executions produced the required ran-cell sentinel; zero tool skips.
The corpus now has four FLOW-MATIC rows and 166 non-ALGOL programs declaring
1182 cells. Hosted CI remains the merge gate.

## VM-036 implementation contract (selected after #14341 merged)

The ten-frontend audit merged as `1fcd588c6b` after all checks passed on
`23dc4ec216`. No new runtime failure supersedes the coverage queue. Select
VM-036: McCarthy's native capstone should execute on Windows and Linux, not
return None solely because the host is not macOS. ALGOL remains separately owned.

Use each host's existing executable compilation API. Probe the matching linker:
Windows must distinguish Microsoft/LLVM/MinGW from POSIX link.exe; Linux honors
CC as the production linker does; macOS probes ld. Once the linker is detected,
compile/link/launch or result errors must fail loudly. Preserve the existing
19-program corpus and exact result comparisons (including process failures).

Add a native-only test over all 19 programs for the existing Windows native CI
step. It must assert nonempty/full execution, emit a count, and fail if
LANG_REQUIRE_WINDOWS_AOT=1 but no Windows linker is found. Normal Linux/macOS
BUILD already runs the conformance target. Validate the focused corpus locally
with the real Microsoft linker and LLVM linker, and prove the required-linker
negative path fails with an empty PATH in a child invocation. Update coverage
docs and retain all existing standard/managed conformance lanes.

VM-036 local validation: all 19 native programs executed with Microsoft
link.exe, then all 19 with an LLVM-only linker PATH. With an empty PATH and
LANG_REQUIRE_WINDOWS_AOT=1, the same test failed at the required-linker assertion
(exit 101, zero executed programs). All-target lang-aot Clippy with warnings
denied and six Windows CI selector tests passed. Linux/macOS execution and the
new hosted Windows command remain required PR CI evidence before merge.

## VM-027 implementation contract (selected after #14333 merged)

PR #14333 merged as `cd73f3ad86` after Linux, macOS, Windows and the CI gates
passed on `422ad4c847`. No new runtime defect was exposed by VM-035. With
VM-025 still separately owned, reprioritization selects the ten-frontend
feature/backend inventory before new Oct or Nib machine semantics.

Produce a source-linked inventory for every wired Language variant. Distinguish
frontend implementation, declared executable corpus coverage, clean refusals,
missing tests and absent-tool skips; a wired backend is not full feature parity.
Include dedicated McCarthy and Macsyma suites, platform restrictions and exact
normal BUILD commands. Compare frontend lowerers/oracle tests to the matrix
rather than inferring completeness from README headings or row counts. Preserve
ALGOL ownership and record its full-matrix gate separately.

Correct stale driver and conformance scope comments. Log each uncovered feature
family as a bounded follow-up with an executable acceptance criterion, grouping
only features sharing a lowering/runtime boundary. Prioritize observed failures
and missing CI protection before expanding semantic scope. Validate inventory
links and corpus counts against source; execute representative dedicated-suite
proofs for the coverage outside the unified corpus, reporting actual skips.

## VM-035 implementation contract (selected after #14325 merged)

Local validation: the new isolated-target regression failed against the old
helper with a nonexistent default archive, then passed after Cargo artifact
discovery was implemented (three parent/parser tests; the ignored child is
explicitly executed by its parent). The original Twig heap cell `2:Llvm`
compiled and ran with its single-cell sentinel in both a fresh target directory
containing spaces and the default directory. All-target lang-aot Clippy passed
with warnings denied. Current-head hosted CI remains the merge gate.

PR #14325 merged as `bf9043d695` after all applicable checks passed on
`618579d847`. Reprioritization selects the reproduced archive lookup defect
VM-035 before the larger VM-027 feature audit; VM-025 remains separately owned.

The shared `lang-aot/tests/common` GC archive helper must obtain the staticlib
path from the successful nested Cargo build's JSON compiler-artifact messages.
Match the `gc_core_capi` staticlib target and its `.a`/MSVC `.lib` archive,
excluding rlibs, dynamic libraries and import libraries. Reject malformed,
missing or ambiguous archive messages and nonexistent output paths; never fall
back to a guessed workspace/target filename. Preserve actionable compiler
failure diagnostics and honor Cargo's target-directory/configuration choices.

Use the existing Cargo build as the authority, with a test-only JSON dependency.
Add a normal BUILD regression that launches a child test process with a fresh
`CARGO_TARGET_DIR` containing spaces. It must build and find the actual archive
under that directory, without mutating the parent test process environment.
Verify JSON selection for Unix and MSVC names and unrelated/no/ambiguous
artifacts. Confirm the original heap LLVM matrix cell runs in a fresh process
with a nondefault target directory. Preserve default-layout use, other test
runners, production runtime semantics and separately owned ALGOL behavior.

## VM-026 implementation contract (selected after #14317 merged)

PR #14317 merged as `6fcc258b32` after every applicable current-head check
passed on `763e68acaf`. VM-024 is complete. VM-025 remains with the separate
ALGOL campaign, so select the highest-priority unowned item, VM-026. The
alternate-target helper issue VM-035 remains recorded for its own bounded fix.

Reconcile `LANG-FULL-IMPLEMENTATION.md`, `LANG-PLATFORM-MATRIX.md`, and the
umbrella `LANG-PLATFORM-VISION.md` with current source and executed evidence.
Correct stale Twig symbol/quote, forward-global, and boxed-global arithmetic
gaps and the obsolete VM-014 work pointer. Distinguish ten wired Language
variants from the eight languages represented in the unified corpus and the
dedicated McCarthy/Macsyma suites. Describe seven standard engines separately
from additional declared BEAM cells. Retain historical LM0 milestones as
history, not claims of full current language completion. Fix the directly linked
E6 dispatch status where it repeats the same already-landed gap.

Link current runner/test/BUILD boundaries and the merged runtime/coverage PRs.
Keep genuinely unverified feature coverage under VM-027, platform vision work
under VM-029, and separately owned ALGOL semantics unchanged. Validate source
counts, named regression rows, local document links and diff cleanliness;
reuse #14317's executed 163-program/1,161-cell evidence rather than inventing
new completeness claims or redundant runtime tests for a documentation edit.

## VM-024 implementation contract (selected after #14295 merged)

PR #14295 merged as `b4535bf5ab` after final head `17a5faa492` passed all
applicable checks, including actual execution of the dedicated Windows native
step. With VM-030, VM-033, VM-034, and VM-032 landed, reprioritization returns to
VM-024 as the highest remaining unowned coverage item.

Add a normal BUILD invocation selecting all non-ALGOL rows from the canonical
`PROGRAMS` corpus and every backend each row declares. Preserve the complete
matrix and diagnostic single-cell protocol unchanged. Use the existing strict
runner and result assertions, including the portable text comparison. Count
programs, executed cells and missing-tool skips; reject empty selections and
any runner returning no result after its toolchain was detected. Keep this
fail-fast so one in-process failure cannot contaminate subsequent results.

Run the entire selection on the local host and let normal PR builds validate
Linux/macOS. Windows's dedicated native proof remains VM-032; this item does
not claim the whole language matrix runs in the Windows Rust-only CI leg.
If execution exposes another runtime failure, record its exact program/backend,
confirm it in a fresh single-cell process, and promote a bounded repair before
publishing the wider coverage gate. Full ALGOL CI remains separately VM-025.

Local Windows validation passes the complete selected corpus: 163 programs,
1,161 executed backend cells, zero skips, in 351.52 seconds. All-target
`lang-aot` Clippy and security review pass. The first alternate-target-layout
attempt is recorded as VM-D025/VM-035; the passing result uses Cargo's normal
workspace target directory. Hosted current-head CI remains the merge gate.

## VM-032 implementation contract (selected after #14289 merged)

PR #14289 merged as `e6765d7711` after all applicable checks passed on
`92250cbe12`. VM-033 and VM-034 are complete. Reprioritization now selects
VM-032 ahead of VM-024: Windows must execute its native regression on affected
PRs before widening the general matrix coverage.

Derive a dedicated LANG Windows runtime flag from the build plan's Windows
`affected_packages` override, falling back to the top-level affected closure
only when there is no Windows override. Select `rust/twig-aot` or
`rust/lang-aot`; the planner already propagates changes through their dependency
closure. A null or missing closure means full coverage; an empty closure means
no affected packages. Reject malformed plans. Changes to the workflow, selector,
its tests, or the MSVC bootstrap must self-select the gate even for an empty
package plan.

Wire the flag into Windows runner selection, Rust setup, and the existing MSVC
developer-environment setup. Add a dedicated Windows step running
`cargo test -p twig-aot --test windows_x86_64_smoke -- --nocapture` with
`LANG_REQUIRE_WINDOWS_AOT=1`. In that mode absence of a real linker must fail,
not silently skip. Preserve optional local-toolchain skips without the flag and
the two explicit, separately tracked precise-GC early returns (VM-031).

Validate selection against Windows overrides, unrelated and full plans, and
self-changes. Exercise the real Windows suite with the required flag, then run
its scalar test binary with no linker on PATH and prove a nonzero result.
Validate workflow wiring and the existing planner/metadata contracts. Actual
hosted Windows runtime execution on the PR is the final acceptance evidence.

Local validation: all eight smoke tests report success with the required flag;
five launch programs, one checks the PE object, and two explicitly return for
VM-031. Removing every linker from PATH makes the scalar test fail with exit
101 in required mode, while optional local mode retains its documented skip.
Six selector/wiring tests, 15 CI-registry tests, and five MSVC-bootstrap tests
pass. The recorded #14265 plan selects this gate, Ruff and YAML parsing pass,
and all-target `twig-aot` Clippy is clean. Hosted CI remains the merge gate.

## Ranked backlog

### VM-033 implementation contract (selected after #14265 merged)

PR #14265 merged as `0e18482307` after every applicable current-head CI check
passed. Reprioritization selects the independently reproduced `352:Llvm`
stdout mismatch before VM-032 and VM-024 coverage work.

`Expect::Stdout` describes text lines for the text-output languages: compare
CRLF as LF, on either host, without removing lone carriage returns, spaces,
tabs, empty lines, or Unicode characters. Brainfuck is the byte-oriented
exception and retains the existing exact comparison. Keep the raw observed
stdout in failure diagnostics. Apply the same comparison boundary to the BASIC
full-value differential tests, so the matrix and differential harness agree.
This changes test interpretation only; do not change compiler or runtime output.

Validate positive LF/CRLF equivalence and negative cases for altered content,
lone CR, and Brainfuck output. Execute the existing BASIC multi-line real,
mixed DATA, and RND rows on every available declared backend, requiring every
available runner to return a result. Add those focused tests to normal BUILD;
the complete non-ALGOL corpus remains VM-024 and full ALGOL coverage VM-025.

The new RND regression exposed VM-034 before publication. The focused repair
must include this prerequisite rather than excluding that backend or program:
the JVM's simulator-compatibility pass must preserve integer widths in modules
that use floating-point values. Such modules cannot run on the legacy 32-bit
integer-only simulator anyway. Keep this decision module-wide so helper calls
and globals agree; prove that RND's `i64` product remains wide and execute the
existing seed/repeat/advance transcript on real Java. This is a generic mixed
numeric-type boundary, not a BASIC-specific RNG special case. Preserve the
existing integer-only simulator path and run its regression suite.

Local validation on Windows: the LF/CRLF regression fails before the fix and
passes after it; three focused tests pass with 18 executed backend cells and
three missing-tool skips. The library and five JVM suites pass 24 tests,
including the integer-only simulator and real-Java Lisp tests. BASIC T7 tests
exercise 400 generated programs with 864 cross-engine agreements. The ALGOL
real-procedure and real-for-variable matrix regressions pass on available
backends, and all-target Clippy is clean. Full-matrix completion is not claimed.

| Rank | ID | Status | Work item | Completion proof |
|---:|---|---|---|---|
| — | VM-001 | done ([#13306](https://github.com/adhithyan15/coding-adventures/pull/13306)) | Preserve COBOL scale-12 decimal intermediates on JVM. | The existing `A / B + C` LANG matrix cell prints `000533` on real Java and the full matrix is green. |
| — | VM-004 | done ([#13340](https://github.com/adhithyan15/coding-adventures/pull/13340)) | Normalize signed process results in `macsyma_conformance` for LLVM and native AOT. | `-7$` compares as `-7`, not process status 249, and the full Macsyma conformance suite passes on every available backend. |
| — | VM-005 | done ([#13348](https://github.com/adhithyan15/coding-adventures/pull/13348)) | Link Linux LLVM matrix executables with the host math library. | Dartmouth BASIC `trunc` and non-ALGOL math cells link and pass under `lang_matrix` on Linux; ALGOL-native gaps remain separately owned. |
| — | VM-002 | done ([#13326](https://github.com/adhithyan15/coding-adventures/pull/13326)) | Protect every CI-green `lang-aot` integration suite in `lang-aot/BUILD`, including `e6d7a_wasm_closures`; replace stale exclusions with the current Macsyma and Linux-matrix failures. | The normal build command runs every CI-green top-level integration target and documents both red exclusions precisely. |
| — | VM-003 | done ([#13356](https://github.com/adhithyan15/coding-adventures/pull/13356)) | Reconcile `LANG-FULL-IMPLEMENTATION.md` with landed Twig E6 and McCarthy work. | Roadmap statuses match current executed suites and name only genuine gaps. |
| — | VM-015 | done ([#13367](https://github.com/adhithyan15/coding-adventures/pull/13367)) | Reconcile the McCarthy compiler README with the completed L3/W16 backend campaign. | The package README no longer calls L3 future work and links the authoritative eight-backend completion matrix. |
| — | VM-010 | done ([#13380](https://github.com/adhithyan15/coding-adventures/pull/13380)) | Close the remaining Twig dynamic VM/JIT gaps: interned symbols/quote and forward-referenced globals, including boxed global arithmetic. | The existing symbol and dynamic-global matrix cells run on VM and JIT in addition to the five code-generation backends. |
| — | VM-011 | decomposed | Close Dartmouth BASIC's non-ALGOL semantic tail: remaining math builtins and dynamic string data paths. | Split into VM-016 through VM-018 because deterministic transcendental wiring, mixed-type `DATA`, and portable randomness have independent designs and risk. |
| — | VM-016 | done ([#13392](https://github.com/adhithyan15/coding-adventures/pull/13392)) | Wire Dartmouth BASIC's deterministic `SIN`, `COS`, `LOG`, and `EXP` builtins to the existing shared transcendental IIR ops. | One discriminating program executes all four functions on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. |
| — | VM-019 | done ([#13419](https://github.com/adhithyan15/coding-adventures/pull/13419)) | Reconcile the Dartmouth BASIC compiler README with landed general exponentiation, seven-backend string arrays, and current dynamic-string limitations. | The README no longer directs work toward already-landed power or string-array support and names only executed current gaps. |
| — | VM-014 | decomposed | Extend the unified matrix to every wired frontend, especially FLOW-MATIC and Macsyma/JIT. | Split into VM-020 and VM-021 because FLOW-MATIC needs an executed matrix baseline while Macsyma needs new universal-JIT runtime glue. |
| — | VM-020 | done ([#13428](https://github.com/adhithyan15/coding-adventures/pull/13428)) | Add FLOW-MATIC's first unified matrix baseline. | A field `MOVE` plus `WRITE-ITEM` program prints `0` on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. |
| — | VM-021 | done ([#13442](https://github.com/adhithyan15/coding-adventures/pull/13442)) | Add Macsyma to the universal JIT and its cross-backend conformance suite. | Macsyma's integer arithmetic corpus agrees across VM, NativeAOT, LLVM, WASM, JVM, CLR, and JIT. |
| — | VM-017 | done ([#13452](https://github.com/adhithyan15/coding-adventures/pull/13452)) | Add mixed numeric/string Dartmouth BASIC `DATA`, `READ`, and `RESTORE` semantics. | Scalar and array string reads preserve source order with numeric values and execute on all applicable standard backends. |
| — | VM-022 | done ([#13762](https://github.com/adhithyan15/coding-adventures/pull/13762)) | Repair the TypeScript Dartmouth BASIC parser `BUILD` so it runs that package's tests instead of ending in the generic parser package. | `BUILD` executes the Dartmouth parser suite itself and includes a mixed numeric/string `DATA` regression. |
| — | VM-023 | done ([#13773](https://github.com/adhithyan15/coding-adventures/pull/13773)) | Audit and repair the same stateful-directory build defect across non-ALGOL TypeScript parser and lexer frontends. | Every affected package's normal and Windows build scripts run that package's own tests, with an automated guard against ending in a dependency directory. |
| — | VM-012 | done ([#13785](https://github.com/adhithyan15/coding-adventures/pull/13785)) | Implement Nib BCD storage semantics and Intel-4004 RAM mapping. | Hardware-faithful programs agree across the portable matrix and the 4004 simulator. |
| — | VM-018 | done ([#13802](https://github.com/adhithyan15/coding-adventures/pull/13802)) | Define and implement portable Dartmouth BASIC `RND` semantics. | Merged as `8ceb8efc60`; the matrix includes negative reseeding, positive advancement, zero repeat, and shared `DEF FN` state on seven backends. |
| — | VM-030 | done ([#14265](https://github.com/adhithyan15/coding-adventures/pull/14265)); discovered by VM-024 | Repair Windows native-AOT CRT linking. | Merged `0e18482307`; both Windows linkers execute the smoke suite, 62 unit tests and Clippy pass, and Linux/macOS/Windows CI and both CI gates finished green. |
| — | VM-033 | done ([#14289](https://github.com/adhithyan15/coding-adventures/pull/14289)) | Define portable text-output comparison in the LANG matrix. | Windows LLVM BASIC cell `352:Llvm` prints correct values with CRLF but the LF expectation fails; normalize only the accepted host text-newline difference, preserve meaningful output bytes, and run discriminating multi-line cases on available backends. |
| — | VM-034 | done ([#14289](https://github.com/adhithyan15/coding-adventures/pull/14289)) | Preserve JVM integer arithmetic in mixed floating-point modules. | RND cell `360:Jvm` must produce `22`, `85032`, `85032`, `601352`; its `48271 * 48271` intermediate stays i64, while the existing integer-only simulator tests remain green. |
| — | VM-032 | done ([#14295](https://github.com/adhithyan15/coding-adventures/pull/14295)) | Protect LANG native Windows execution in PR CI. | An affected `twig-aot`/LANG dependency change selects an actual Windows executable smoke run, with its toolchain present; assert real execution rather than a green Clippy-only job. Preserve platform-plan gating and keep the known GC early returns explicit. |
| — | VM-024 | done ([#14317](https://github.com/adhithyan15/coding-adventures/pull/14317)) | Protect the non-ALGOL language matrix in normal CI. | Every non-ALGOL `PROGRAMS` row executes on each available declared backend; no empty selection or silent failure-to-skip conversion; full ALGOL diagnostics remain available. |
| 2 | VM-025 | queued; coordinate with ALGOL owner | Reproduce the full matrix's current Linux failures and restore full CI coverage after their repair. | Record exact failing cells and owning PRs; remove the full-matrix exclusion only after the complete target runs green on supported CI hosts. |
| — | VM-026 | done ([#14325](https://github.com/adhithyan15/coding-adventures/pull/14325)) | Reconcile stale Twig, frontend-count, and completed-work claims in LANG-FULL and LANG-PLATFORM status documents. | Every remaining gap links a current source/test boundary; landed VM-010, VM-017, VM-018, VM-020, and VM-021 work is no longer described as missing. |
| — | VM-035 | done ([#14333](https://github.com/adhithyan15/coding-adventures/pull/14333)) | Honor Cargo target-directory overrides in shared AOT test archive discovery. | With a nondefault `CARGO_TARGET_DIR`, locate the archive Cargo actually built and execute a heap LLVM cell; never return a nonexistent guessed path. |
| — | VM-027 | done ([#14341](https://github.com/adhithyan15/coding-adventures/pull/14341)) | Audit feature-by-backend coverage for all ten wired frontends, including dedicated McCarthy and Macsyma suites. | Inventory implemented features, declared/refused backend cells, executable proofs, and CI commands; split every uncovered implemented feature into a bounded parity item. |
| 5 | VM-013 | design required | Define portable semantics for Oct's Intel-8008 intrinsics (`in`, `out`, `adc`, `sbb`, rotations, carry, parity). | Document machine state and I/O contracts, then split into PR-sized operations with both portable and real 8008-simulator proofs; resolve only essential language-design questions with the user. |
| 6 | VM-028 | queued | Audit Nib's remaining Intel-4004 arithmetic and control-flow parity beyond BCD storage. | Compare implemented operations against the 4004 backend and simulator; file bounded missing-operation or refusal tests and close them with executed proofs. |
| 7 | VM-029 | queued | Audit the broader platform vision and native-runtime roadmaps into explicit completion milestones. | Separate landed capabilities from GC, tooling, IR-bridge/transpilation, and measured-performance work; give every retained milestone a testable acceptance criterion and owner. |
| 7a | VM-031 | queued under VM-029 | Reconcile and complete native precise-GC frame-walk proofs. | Windows smoke has two explicit early-return differentials for the Rust/Twig frame boundary; inventory supported-host GC gaps, track each refusal, and execute live-byte reclamation proofs before calling native GC complete. |

## VM-027 audit discoveries and next priorities

The [feature/backend inventory](LANG-VM-FEATURE-COVERAGE.md) compares ten
frontends with the eight-language unified corpus and dedicated suites. Local
McCarthy (19 x 7) and Macsyma (21 x 7) capstones passed. No executed red cell
was discovered. Missing existing-feature proofs therefore take priority over
new machine semantics. After this audit merges, select VM-036 first: existing
native code should be exercised on Windows/Linux before expanding coverage.
VM-025 remains separately owned. The following queue precedes VM-013/028/029;
items requiring new runtime lowering follow the coverage-only promotions.

| Order | Item | Bounded work and executable acceptance |
|---|---|---|
| done #14349 | VM-036 | Run McCarthy's existing 19-program native capstone on Windows/Linux as well as macOS. Assert a present linker cannot silently skip; wire actual Windows execution into CI and prove all 19 results. |
| done #14358 | VM-037 | Promote FLOW-MATIC compare/branch/jump behavior beyond the scalar-output baseline. Use terminating, discriminating output programs on all seven standard columns. |
| done #14363 | VM-044 | Promote Oct while/loop/break and returned function values to observable seven-column programs; prove u8 wrap and actual branch/call effects. |
| done #14370 | VM-045 | Promote COBOL reference modification to standard columns: constant and dynamic bounds, result text and explicit invalid-bound behavior, compared with its existing oracle. |
| done #14394 | VM-046 | Promote COBOL STRING/UNSTRING in separate slices for SIZE, delimiters, pointer and overflow behavior; each slice needs oracle-matched output on its declared code-generation columns. |
| done (see PR below) | VM-047 | Promote COBOL INSPECT in separate tally, replacement and region slices; preserve first-match/non-rechaining and documented character boundaries; compare executed outputs with the oracle. |
| done (see PR below) | VM-049 | Add a real .NET lane for the existing Macsyma arithmetic corpus with explicit tool gating and full result assertions; preserve the simulator floor. |
| done #14471 | VM-038 | Probe Macsyma v0 integer arithmetic/assignment on BEAM and add a real Erlang corpus lane, or record a precise unsupported lowering with a regression before a separate fix. |
| done #14601 | VM-039 | Define portable FLOW-MATIC input_more/EOF semantics, then run a finite read/process/write stream on each code-generation column; no post-detection failure-to-skip conversion. |
| 10 | VM-040 | Inventory remaining BEAM cells separately for Twig strings, Twig records/closures, Nib scalars, BASIC f64/I/O, Oct u8/I/O, FLOW-MATIC and COBOL. Each family first gets a discriminating probe; split actual lowering defects before implementation. Brainfuck remains the explicit excluded tape design. Nib, Oct and COBOL-60 are now fully BEAM-declared; Dartmouth BASIC's 18-row pure-string family is BEAM-declared, its remaining ~28 numeric rows need new `iir-to-beam` f64 lowering (VM-D033), and its 5 `INPUT` rows plus FLOW-MATIC's 4 `INPUT`/EOF rows need VM-060b. |
| done (see PR below) | VM-042 | ~~Pin Brainfuck's intentional BEAM exclusion with a driver-level error assertion for mutable tape operations~~ — the premise was stale (VM-D031): tape mutation already works via `iir-to-beam`'s `:atomics` ops. Promoted the three non-input rows to real BEAM cells instead, and pinned the genuinely remaining `getchar`/host-input refusal at both the `lang-aot` and `iir-to-beam` layers; distinguishes supported frontend compilation from backend refusal, correctly scoped to input only. |
| 12 | VM-041 | Isolate Twig captured/reassigned runtime-string lowering from existing source-local string metadata; add one captured-string value proof before wider dynamic-string expansion. |
| 13 | VM-048 | Define a representation-neutral observation for Macsyma's implemented inert symbolic Apply, then promote one oracle-derived symbolic result per backend; do not compare raw pointer/tag identities. |
| 14 | VM-058 | Implement genuine COBOL INSPECT `BEFORE x AFTER y` two-delimiter window intersection on a single delimiter phrase (discovered as VM-D027): both the `cobol-runtime` oracle and the compiler currently read only the first of two grammar-legal `inspect_region` siblings. Touches all nine region-parsing call sites (TALLYING/REPLACING/CONVERTING, single- and multi-item) in both engines; add a discriminating two-distinct-delimiter proof (present/present, one absent) plus a matrix cell once implemented. |
| done (see PR below) | VM-061 | Discovered as VM-D030: `LANG-VM-FEATURE-COVERAGE.md`'s per-frontend "Declared standard cells" table undercounts the real corpus by roughly 66 cells (a fresh full-matrix run reports 1548 declared non-ALGOL cells; the table's rows summed to 1478 before the COBOL-60 row's independent correction to 422). Recompute every frontend's row/cell count directly from `lang_matrix.rs` — ideally via a small checked script or test assertion so the table cannot silently drift again — reconciling Twig, Nib, Brainfuck, Dartmouth BASIC, Oct and FLOW-MATIC against source. |

VM-047c (region proofs, including BEFORE/AFTER used together across a
combined statement's independently-regioned TALLYING/REPLACING halves)
completed VM-047. The genuine, separate "single phrase carries both BEFORE
and AFTER" intersection gap it exposed is VM-058, ranked with the other new-
frontend-semantics items above rather than blocking VM-047c.

VM-049 (real CoreCLR execution for the Macsyma arithmetic corpus) is complete:
`tests/clr_real_macsyma.rs` proves the identical 21-program corpus the
in-repo-simulator floor already agrees on, and the VM-D028 fix
(`# needs-toolchain: dotnet` in `lang-aot/BUILD`) makes hosted PR CI actually
install `ilasm` for it — and retroactively for McCarthy's pre-existing
`clr_real_*` lane, which had the identical silent gap. No executed red cell or
CI-protection gap outranks the next item, so **VM-038** (a real BEAM lane for
Macsyma, mirroring what VM-049 just did for CLR) is next per this ordering,
unless hosted CI on VM-049's own PR surfaces a red cell first — the first
environment able to actually prove or disprove real CoreCLR execution for
either CLR-real lane.

BASIC two-dimensional numeric arrays were verified in the matrix and lowerer,
so their stale README description is corrected here rather than creating a new
implementation item. The known DEF FN-global and print-zone semantics remain
future frontend design scope, not missing proofs for already-implemented code.

## Discovery log

- **VM-D035 — CONFIRMED AND FIXED 2026-09-15 (#15265):** while scoping
  VM-041 (Twig BEAM closure/string/record isolation), a prior agent flagged
  and this session independently confirmed by direct code read:
  `iir-to-beam`'s `"call_closure"` lowering arm (`src/lower.rs`) emits TWO
  `OP_CALL_EXT` instructions — `erlang:'++'/2` (combining captured values
  with call arguments), then `erlang:apply/3` (invoking the closure) — and
  neither was wrapped in the `save_live_across_imported_call!`/
  `restore_live_across_imported_call!` macro pair every other call-emitting
  op in this file uses, even though `call_closure` was already listed in
  the `live_across` liveness match (whose own comment names the exact bug
  class, VM-D029, already fixed once for the six `:atomics` ops: an op
  missing the save/restore wrap has its live variables silently destroyed —
  a WRONG VALUE, not a crash). Confirmed CURRENTLY DORMANT: no
  `lang_matrix.rs` row exercising `call_closure` had ever declared `Beam`,
  so nothing in CI had exercised the gap. **Fixed in #15265**: both
  `call_ext` emissions are now wrapped in a single save/restore pair
  spanning both calls (mirroring the `array_set` f64/ets arm's existing
  two-call-one-pair pattern), with the call result moved into the
  destination register before the restore runs (mirroring `alloc_array`'s
  ordering, since restore can itself write into `x0`). Pinned by
  `iir-to-beam`'s `test_98_real_erl_call_closure_survives_live_across_call`,
  verified to fail with the wrong value (`14` instead of `106`) when the fix
  is reverted — direct proof the test exercises the bug, not a passthrough.
  See this backlog's top-of-file "VM-041" section for the full fix and the
  probe-first sweep it unblocked (27 of 29 remaining Twig rows promoted to
  `Beam` in the same PR).

- **VM-D033 — confirmed 2026-09-13:** with all 58 COBOL-60 rows BEAM-declared
  (#15099/#15119), a fresh reprioritization asked whether any OTHER non-ALGOL
  frontend still had undeclared BEAM rows the way COBOL did — a question
  nobody had actually re-derived from source since VM-061 (which reconciled
  Twig's counted cells, not every frontend's Beam-declared row count). A
  string-escaping-aware parse of every `Prog` entry, cross-checked against
  the compiled `feature_coverage_doc_counts_match_programs_source` test,
  found Dartmouth BASIC alone at 0/51 declared Beam rows — the only non-ALGOL
  frontend with a *total* BEAM gap, despite "BASIC f64/I/O" being named in
  VM-040's original 2026-09 family list from the start; every other family in
  that list (Nib, Oct, FLOW-MATIC's non-input half, COBOL) had since been
  addressed while BASIC was simply never picked up. A `compile_source_to_beam`
  probe over all 51 rows found the root cause: `iir-to-beam` has zero `f64`
  lowering support (confirmed by reading `lower.rs` end to end — full
  string-op coverage, no float arm at all), and BASIC's `BA7-1b` change
  routes every scalar numeric value, even integer-spelled literals, through
  the shared `f64` track, while the frontend also unconditionally emits its
  `__basic_print_real`/`__basic_print_fixed_mag`/`__basic_print_real_e`
  helpers into every compiled module regardless of reachability — so
  `iir-to-beam`'s whole-module float-const validation rejects every numeric
  BASIC program, even ones that never print a float themselves. Exactly 18
  rows (the entire purely-string-valued family — no numeric `PRINT`/`LET`/
  `FOR`/`INPUT` in source, so no float `const` is ever emitted) compile
  cleanly and need no new lowering. **Resolved for those 18** (see the
  top-of-file VM-040 Dartmouth BASIC BEAM pure-string family section): all 18
  promoted and executed correctly on real `erl`. **Not resolved** for the
  remaining ~28 numeric rows (needs new `iir-to-beam` f64 lowering — its own
  properly scoped design/implementation item, materially larger than a
  same-day promotion slice) or the 5 `INPUT` rows (needs VM-060b, shared with
  FLOW-MATIC's 4 blocked `INPUT`/EOF rows).

- **VM-D032 — confirmed 2026-09-11:** while probing the next COBOL BEAM
  reference-modification rows for promotion (the first two to require
  `Expect::Trap` on BEAM), found `iir-to-beam`'s `str_slice` lowering never
  enforced its own documented bounds-check contract. `str_slice` lowers to
  `lists:sublist(List, Start+1, Len)`; `sublist` is far more lenient than
  the contract every other backend's `str_slice` proves
  (`vm-core::dispatch::handle_str_slice`: trap when `start < 0 || end <
  start || end > len`) — when `Start` is in range but `Start + Len - 1` runs
  past the source, `sublist` silently returns a SHORT list instead of
  raising. A scratch probe against real `erl` confirmed it directly:
  `sublist("ABCDE", 4, 5)` (asking for `[3, 8)` of a 5-character string)
  returned `"DE"` (2 characters), exit 0, no error — not a bounds trap. Only
  `Start =< 0` happened to raise on its own, an accident of `sublist`'s own
  guard clauses (`Start >= 1` required), not a deliberate check. No COBOL
  BEAM row, and no other language's BEAM row, had reached an out-of-bounds
  `str_slice` input before this slice, so nothing in CI had ever exercised
  the gap. **Resolved 2026-09-11** (see the top-of-file VM-040 reference-
  modification MOVE/trap section): `str_slice`'s lowering now checks all
  three predicates explicitly (`erlang:length/1` + `is_ge` branches,
  reusing the `cmp_*` label-synthesis pattern) and raises
  `erlang:error(badarg)` on any violation before `sublist` ever runs; two
  direct `iir-to-beam` regressions pin both the exact silent-truncation case
  and its exact-boundary control. `run_beam` in `lang_matrix.rs` also
  gained `Expect::Trap` handling — the one BEAM-runner gap left after every
  other backend's runner already had it, simply because no BEAM row had
  ever needed it before.

- **VM-D031 — confirmed 2026-09-10:** while checking VM-042's filed premise
  ("Brainfuck's intentional BEAM exclusion... for mutable tape operations")
  against source before implementing it as written, found `iir-to-beam` PR
  #11343 (2026-08-13) had already added `:atomics`-backed `alloc_bytes`/
  `store_byte`/`load_byte`, explicitly "unblocking Brainfuck (which needs a
  mutable tape)" per that PR's own description — three weeks BEFORE VM-042's
  backlog text was written (2026-09-05) and roughly four weeks after
  `brainfuck-iir-compiler/README.md`'s "Why no BEAM target?" section
  (2026-05-22) had declared the exclusion permanent and by-design. A scratch
  probe compiling all six Brainfuck matrix programs through
  `lang_aot::compile_source_to_beam` and executing the result on real `erl`
  confirmed the three non-input programs already run correctly (byte-
  identical stdout to every other backend) with zero backend code changes,
  while the three STDIN programs cleanly refuse with `UnsupportedOp: …
  "getchar" is not in the BEAM builtin set` — `getchar` was simply never
  added when the memory ops landed, since that PR's motivating tape-loop
  workload never needed it. So the exclusion was real and correctly
  documented for about three months, then silently stale for about four
  weeks, and nobody had re-probed it since. **Resolved 2026-09-10** (see the
  top-of-file VM-042 section): promoted the three non-input rows to a real,
  `erl`-executed `Beam` cell each (42 → 45 declared Brainfuck cells);
  corrected the README and `LANG-VM-FEATURE-COVERAGE.md`; pinned the
  genuinely-remaining `getchar` refusal as a targeted, named backend
  rejection (not a generic or accidental one) at both the `lang-aot` and
  `iir-to-beam` layers, distinct from the frontend, which compiles `,`
  without complaint.

- **VM-D030 — confirmed 2026-09-09:** while updating
  `LANG-VM-FEATURE-COVERAGE.md`'s COBOL-60 row for the condition-name/
  EVALUATE BEAM slice, a fresh `non_algol_matrix_every_proven_cell_agrees`
  run reported 1338 cells exercised + 210 skipped = 1548 declared non-ALGOL
  cells, versus the document's pre-existing rows summing to 1478 (already
  short by 66 cells before this slice's own +4 COBOL cells). The COBOL-60
  row itself is independently verified accurate (58 rows, 422 cells,
  confirmed against source twice with different parsing approaches); the
  other frontend rows were not re-audited here. The grand-total line is
  corrected to the freshly measured 1548 per this document's own authority
  rule. Queued as **VM-061**: recompute every row from source, not by hand.
  **Resolved 2026-09-09** (see the top-of-file VM-061 section): a
  brace-balanced parse of every `Prog` in `PROGRAMS` found only Twig's row
  actually undercounted (343 vs. the true 363 — it had excluded its own 20
  Beam cells, unlike Nib/Oct/FLOW-MATIC/COBOL-60, whose doc numbers already
  counted Beam in their totals); every other non-ALGOL row's row/cell count
  already matched source exactly, so the "roughly 66 cells" estimate above
  was itself a rough text-search guess, not the real gap — the real gap was
  exactly Twig's missing 20 Beam cells. Correcting only that one cell
  (343 → 363) makes the table's non-ALGOL rows sum to exactly 1548, matching
  the fresh full-matrix total precisely. A new
  `feature_coverage_doc_counts_match_programs_source` test in
  `lang_matrix.rs` now pins all seven non-ALGOL row/cell pairs (plus a
  zero-rows check for McCarthy/Macsyma) against live source.

- **VM-D028 — confirmed 2026-09-07:** while adding a real-CoreCLR lane for
  Macsyma (VM-049), investigating why McCarthy's pre-existing `clr_real_*.rs`
  lane had never been observed to report a real pass in hosted CI (only the
  expected local skip, per VM-047c) found `lang-aot/BUILD` never declared
  `# needs-toolchain: dotnet`. `lang-aot`'s bucket language is "rust", and
  `.github/workflows/ci.yml` gates `actions/setup-dotnet` and the `ilasm`
  NuGet restore on the CI-wide `needs_dotnet` flag, which without this
  declaration no PR touching only `lang-aot` ever set. Confirmed against
  #14449 (VM-047c)'s own hosted Linux job log: `needs_dotnet=false`, and the
  `Verify tools` step's `.NET: $(dotnet --version)` line was generated but
  guarded by `if [ "false" = "true" ]`, so it never ran. Every CLR-real test
  still skipped correctly (the tool gate itself was never wrong), but the
  CLR-real column had never actually executed on its own PR merge-gate CI,
  only on a forced main-branch full build. Fixed within VM-049 by adding the
  declaration, the same pattern `java-to-semantic-ir/BUILD` already uses.

- **VM-D027 — confirmed 2026-09-07:** while validating VM-047c's "BEFORE and
  AFTER together" test coverage, a discriminating probe (`INSPECT S TALLYING C
  FOR ALL "0" BEFORE "X" AFTER "X"` over `S = "00X00"`) found that a SINGLE
  delimiter phrase carrying BOTH region keywords parses (`cobol.grammar`'s
  `inspect_region` sits under a `{ }` repetition, producing two sibling
  `inspect_region` nodes) but both engines read only the FIRST node via
  `child_node(_, "inspect_region")` — the oracle's `program.rs` and all seven
  matching call sites in the compiler's `lib.rs` — silently discarding the
  second keyword instead of intersecting the two windows or rejecting the
  phrase as a later rung. The oracle and compiler agree (bare `BEFORE "X"`,
  count 2), so this is a shared, non-diverging limitation, not a cross-engine
  conformance failure; it does not block VM-047c. Real ISO COBOL allows this
  combination to restrict scanning to characters simultaneously after one
  delimiter and before another. Promoted to VM-058, ranked alongside the other
  new-frontend-semantics items; a regression test pins the current
  first-region-only behavior in the interim.


- **VM-D026 — confirmed 2026-09-05:** source/document inspection for VM-026
  confirms ten driver variants but only eight unified corpus languages;
  McCarthy and Macsyma remain dedicated suites. Source doc comments also retain
  older claims (for example Macsyma conformance's BEAM-is-McCarthy-only note).
  The current spec reconciliation states the executed coverage boundary;
  VM-027 must include those source comments when auditing feature/backend
  declarations, rather than treating them as authoritative absence claims.

- **VM-D025 — confirmed 2026-09-05:** VM-024 validation reused a merged
  worktree's Cargo target via `CARGO_TARGET_DIR`. The shared test helper
  `tests/common::gc_core_capi_archive` builds with Cargo's inherited override but
  searches only `<workspace>/target/release`, then returns a nonexistent Unix
  archive name if neither file is there. LLVM Twig `(car (cons 42 0))` fails at
  linking for that reason. Queue VM-035 to derive the actual Cargo artifact
  path. Continue VM-024 validation with the normal target directory; this is
  an alternate build-layout helper gap, not an observed default-layout runtime
  failure and not evidence against a backend's language semantics.

- **VM-D024 — confirmed 2026-09-05:** VM-033's real multi-line/RND proof
  exposed JVM output `22`, `-914968`, `-914968`, `-398672`. A fresh process
  using the pre-VM-033 matrix binary reproduces `360:Jvm`, so normalization
  did not cause it. `concretize_scalar_any_for_jvm` narrows RND's explicit
  `i64` product to `i32` because every literal fits in i32 and BASIC's newer
  floating-point PRINT is not the old `print_i64` builtin. The second RNG
  product overflows before modulo. Promote VM-034 as VM-033's prerequisite;
  keep mixed floating-point modules out of the integer-only simulator rewrite
  and retain all three selected multi-line regressions.

- **VM-D023 — confirmed 2026-09-05:** with VM-030 applied, the proposed
  non-ALGOL matrix ran for 308 seconds before failing on LLVM Dartmouth BASIC
  multi-line real output. Expected `3.14\n.25\n-2.5`, observed
  `3.14\r\n.25\r\n-2.5`. A fresh single-cell process with
  `LANG_MATRIX_ONLY_CELL=352:Llvm` reproduces the mismatch in 0.35 seconds.
  This is a host text-output comparison failure, independent of CRT symbol
  linking. Promote VM-033 ahead of coverage work; retain the exact content and
  control-character contract rather than broadly trimming away discrepancies.

- **VM-D022 — confirmed 2026-09-05:** `.github/workflows/ci.yml` selects a
  Windows job when `needs_rust` is true, but `Build and test affected packages`
  on Windows additionally requires Swift or Elixir. Its dedicated Windows-only
  Rust step is Clippy only and does not select the portable `twig-aot` crate.
  Therefore a green Windows job for a Rust-only CRT repair does not execute
  `windows_x86_64_smoke`. Promote VM-032 as the first coverage follow-up. Local
  runs with both real linkers provide VM-030's Windows runtime evidence while
  that gap is repaired separately.

- **VM-D021 — confirmed 2026-09-05:** executing the proposed non-ALGOL
  matrix on Windows failed on the very first native Twig `42` cell with
  duplicate `__vcrt_InitializeCriticalSectionEx`, defined by both
  `libvcruntime.lib` and `vcruntime.lib`. The static library was added for an
  earlier custom `/ENTRY:main` path; that custom entry was removed by
  `bc2cb05594`, but the static library and its old explanation remained.
  Promote VM-030 ahead of VM-024. Remove the obsolete static CRT selection,
  preserve the compiler's normal startup, and run actual Windows executables
  before enabling additional matrix coverage. VM-024's draft patch is held
  outside the checkout until this prerequisite lands.

- **VM-D001 — confirmed 2026-08-27:** JVM scalar concretization narrowed
  COBOL's explicit `i64` constant `10^12` to `i32`, so nested fixed-point
  division printed `000000` instead of `000533`. Promoted to VM-001.
- **VM-D002 — confirmed 2026-08-27:** `lang-aot/BUILD` still describes the
  closure suite as red even though all five tests pass, and excludes the full
  LANG matrix. Promoted to VM-002.
- **VM-D003 — confirmed 2026-08-27:** the LANG-FULL roadmap still marks Twig
  lists/closures/records/unions and McCarthy backend completion as open despite
  landed executed tests. Promoted to VM-003.
- **VM-D004 — confirmed 2026-08-27:** comparing top-level `lang-aot/tests/*.rs`
  targets with the explicit BUILD command found `macsyma_conformance` omitted
  in addition to the two documented exclusions. Running it exposed LLVM process
  status 249 for the expected signed result -7. Promoted to VM-004 and ranked
  above CI/documentation work as a red executed conformance suite.
- **VM-D005 — confirmed 2026-08-27:** enabling `lang_matrix` in Linux CI exposed
  19 failures: LLVM links without `libm` for cross-language
  `pow`/`floor`/`trunc` calls (including Dartmouth BASIC), while native AOT
  refuses newly proven four-dimensional ALGOL array cases. The linker portion
  is promoted to VM-005; the array refusals remain with the separate ALGOL
  campaign. `lang_matrix` stays explicitly excluded from `lang-aot/BUILD`
  until both streams make it Linux-green.
- **VM-D006 — confirmed 2026-08-27:** the roadmap's E6/Twig checklist predates
  landed seven-engine proofs for list operations, records, unions, and closures,
  and still presents McCarthy backend completion as open even though W16 proves
  F1–F7 on all eight applicable backends. Promoted to VM-003. The audit narrowed
  VM-010 to the two Twig cells that still exclude VM/JIT: symbols/quote and
  forward-referenced dynamic globals (plus the documented boxed-global arithmetic
  follow-up).
- **VM-D007 — confirmed 2026-08-27:**
  `mccarthy-lisp-iir-compiler/README.md` still says L3 is the next phase even
  though the authoritative McCarthy platform matrix marks W1–W16 complete.
  Promoted to VM-015 as a separate package-documentation reconciliation.
- **VM-D008 — confirmed 2026-08-27:** the discriminating Twig forward-global
  arithmetic cell returned the tagged word for 42 (`336`, observed as process
  exit 80) on native AOT and LLVM because `lower_dynamic_arith` boxed the
  helper's result without propagating `ref<any>` through its return/call
  signature. BEAM refused the same representation-only `box` op even though
  Erlang integers are already dynamic terms. Retained inside VM-010 because
  boxed global arithmetic was an explicit part of that item.
- **VM-D009 — confirmed 2026-08-28:** VM-011 combined three independent
  implementation classes. `SIN`/`COS`/`LOG`/`EXP` already exist as shared IIR
  operations on every standard backend and need only Dartmouth frontend wiring;
  string `READ`/`DATA` needs a mixed-type ordered pool; `RND` needs an explicit
  portable RNG contract and new substrate. Decomposed into VM-016, VM-017, and
  VM-018 before selecting the deterministic VM-016 slice.
- **VM-D010 — confirmed 2026-08-28:** the Dartmouth BASIC compiler README still
  calls general exponentiation and the final NativeAOT/JVM/CLR string-array lanes
  future work even though their seven-backend matrix proofs have landed. Promoted
  to VM-019 for a focused package-documentation reconciliation.
- **VM-D011 — confirmed 2026-08-28:** the same README broadly calls richer
  dynamic string expressions future work even though the matrix executes
  `str_concat` over two runtime `INPUT` strings on all seven backends. Retained
  inside VM-019; the corrected limitation is mixed numeric/string `DATA` and
  `READ`, already tracked as VM-017.
- **VM-D012 — confirmed 2026-08-28:** VM-014 combined two different gaps.
  FLOW-MATIC is wired into `Language` and already has package-level JIT and
  backend acceptance tests but no unified matrix cell. Macsyma's separate
  conformance suite executes VM, NativeAOT, LLVM, WASM, JVM, and CLR while
  explicitly excluding the universal JIT because its callback glue is still
  McCarthy-specific. Decomposed into VM-020 and VM-021; selected VM-020 first
  because it closes a zero-coverage frontend without changing runtime semantics.
- **VM-D013 — confirmed 2026-08-28:** VM-021 did not require the anticipated
  McCarthy tagged-runtime callback generalization. Macsyma's v0 integer corpus
  becomes typed arithmetic after the existing `lower_dynamic_arith` pass, while
  generic VM/JIT `box`/`unbox` are identity operations. The dedicated
  `run_macsyma_on_jit` runner therefore adds no language-specific runtime
  callbacks; all 21 programs execute and agree across seven engines.
- **VM-D014 — confirmed 2026-08-28:** VM-017 needs one dynamically ordered
  stream without imposing a new tagged-value ABI on seven backends. Parallel
  kind, `array<f64>`, and `array<str>` pools retain one source index and shared
  `RESTORE` pointer. Every `READ` first bounds-checks the kind array and then
  validates the target type; a mismatch deliberately enters the existing
  cross-backend array-bounds trap contract instead of reading a placeholder.
- **VM-D015 — confirmed 2026-08-28:** the shared Dartmouth BASIC grammar is
  embedded in generated parser artifacts outside the Rust LANG VM frontend.
  CI caught the required Ruby regeneration; auditing the same source boundary
  found Python, Lua, and TypeScript parser artifacts with the old numeric-only
  `DATA` rule. VM-017 regenerates all four alongside Rust so no checked-in
  parser silently disagrees with the authoritative grammar.
- **VM-D016 — confirmed 2026-08-28:** the TypeScript Dartmouth BASIC parser's
  `BUILD` chains relative `cd` commands through its dependencies and finishes
  in `../parser`, so its reported 117 tests belong to the generic parser while
  the Dartmouth package's own 61-test suite never runs. Promoted to VM-022 and
  ranked above new semantics as a missing-test-protection defect.
- **VM-D017 — confirmed 2026-08-31:** VM-022's sibling audit found the same
  top-level, stateful `cd ../...` pattern in 27 TypeScript parser `BUILD`
  scripts and 22 lexer `BUILD` scripts, with 20 and 18 corresponding Windows
  scripts respectively. Promoted to VM-023 for a non-ALGOL fleet audit and an
  automated package-directory guard; it remains separate from VM-022 so the
  Dartmouth repair has a focused executed proof.
- **VM-D018 — confirmed 2026-08-31:** after excluding the separately owned
  ALGOL fronts and the already-repaired Dartmouth parser fronts, VM-023 found
  470 stateful sibling-directory commands across 81 parser/lexer build files.
  Dependency installs now run in subshells, and a CI-discovered repository
  test rejects any future top-level sibling `cd` in non-ALGOL `BUILD` or
  `BUILD_windows` fronts.
- **VM-D019 — confirmed 2026-08-31:** the 4004 simulator models 256 main RAM
  characters plus 64 status characters, while the old backend had no RAM ops
  and compiled functions independently. VM-012 uses the complete 320-nibble
  space and builds a module-wide global-slot map before lowering functions, so
  every function agrees on a static's physical address.
- **VM-D020 — confirmed 2026-08-31:** every standard backend already executes
  typed module globals, cross-function calls, exact `i64` multiplication/modulo,
  and `i64`↔`f64` conversion. VM-018 therefore needs no random host ABI: one
  Park–Miller helper can keep a module-global state shared by `main` and
  `DEF FN`. The accepted contract fixes seed 1 at program start, makes negative
  arguments reseed-and-advance, zero repeat, and positive arguments advance.

## Ownership boundary

Do not select ALGOL items from this backlog while the separate ALGOL agent is
active. Cross-cutting fixes may touch shared infrastructure used by ALGOL, but
must preserve its tests and avoid changing ALGOL semantics or roadmap ownership.

### VM-040 probe discovery: BEAM runner discarded program stdout

After print_i64 lowered, the output assertion saw an empty string. Inspection
found run_beam always returned an empty stdout after parsing its result marker.
Preserve the actual bytes before the result marker as program stdout, with a
parser regression covering output plus return value. Keep result-range and
process failure checks. This harness fix is required for observable BEAM cells.

### VM-040 Oct implementation contract: u8 result width

Further executed probes produced -1 for ~0 (expected255) and300 for200+100
(expected44). BEAM arbitrary-precision arithmetic lacks the frontend's u8
width semantics. Mask u8 arithmetic/unary/bitwise results with255 at lowering;
leave wider integer types unchanged. Rerun all12Oct programs, including wrap,
loops/calls, and complement. Add backend structural coverage for masking and
integer output; preserve existing predicate and wider arithmetic behavior.

### VM-040 Oct validation

All12Oct programs passed on real Erlang; each newly declared Beam cell also
passed in a fresh process with the positive execution sentinel (rows75-83 and
381-383 at this revision). The dedicated corpus checks zero return separately
from stdout, protecting marker separation. The backend suite passed92tests
before the additional width regression; that regression passed20op/type cases.
Clippy for iir-to-beam and lang-aot alltargets passed. Remaining VM040families
are Twig strings/records/closures, Nib, BASIC, FLOW-MATIC and COBOL; re-audit
current declarations before selection. VM060b hostinput and VM0138008 remain.

### VM-040 Nib implementation contract: four-bit result width

The real-BEAM corpus reached Nib's ~15 u4 comparison and returned0 instead
of1: complement was not narrowed to four bits. Extend the existing u8 result
mask to select15 for u4 in all three arithmetic/unary/bitwise families; preserve
u8mask255 and wider integers. Extend the op/type regression to u4. Rerun all26
Nib programs including BCD storage, then promote only proven BEAM cells.

### VM-040 Nib validation

All26NibBEAM cells (rows49-74 at this revision) passed in fresh processes
with positive sentinels and no skips. The dedicated26program corpus passed,
and all12OctBEAM programs passed again after widening the mask selection.
The backend suite passed93tests (18unit70integration5doc), including30op/type
width cases. No full seven-standard-column rerun is claimed. Broader4004
fidelity remains VM028; remaining BEAM families are reprioritized after merge.

### VM-040 FLOW-MATIC output implementation contract

The first real probe refuses putchar in main and the generated integer-printer
helpers. Add putchar by building one character list and calling io:put_chars/1,
reserving two heap words and treating this builtin as an imported-call site
for existing live-register spill/restore analysis. It has one value operand
and no result. Validate actual recursive integer printing/control flow; input
and Brainfuck tape parity remain outside this change.

### VM-040 FLOW-MATIC output validation

The four output/control-flow programs passed on real Erlang and in four fresh
single-cell processes (rows 386-389), each with its positive execution sentinel.
A separate 200-character loop preserved live state and returned 42. All 93
backend tests (18 unit, 70 integration, 5 doc) and Clippy for both affected
packages with all targets passed. FLOW-MATIC now declares 60 cells: four output
programs across eight backends and four input programs across seven. No full
rerun of the seven existing columns is claimed. This proves ASCII integer
printing only; byte truncation/encoding, BEAM input/EOF, and Brainfuck tape
semantics still need separate probes before promotion. Reprioritize the remaining
VM-040 families against VM-060b host input and VM-013 Intel 8008 after merge.

## VM-040 COBOL BEAM output probe (selected after #14665 merged)

PR #14665 merged as 9af6235015 after all 46 checks succeeded or skipped.
Prioritize COBOL's first four output cases because they exercise the newly
available character writer together with existing string and integer lowering.
Probe literal DISPLAY, numeric MOVE, integer arithmetic and scaled-decimal ADD
on real Erlang. Preserve exact stdout and hard failures after runtime detection.
Only promote executed cells. If a defect appears, commit its bounded contract
before production edits. Larger COBOL string operations, BEAM input/EOF,
VM-060b host-reader ABI, and VM-013 machine semantics remain separate items.

### VM-040 COBOL probe: integer arithmetic literal operands

Literal DISPLAY and numeric MOVE execute successfully, but ADD 7 fails lowering
with expected variable operand, got Int(7). Integer binary arithmetic currently
requires register sources even though IIR permits integer immediates. Accept
Var and Int operands directly in the existing gc_bif2 arithmetic source slots;
retain explicit errors for unsupported operand kinds, source arity validation,
and u4/u8 narrowing. Do not allocate scratch registers for immediate values.
Rerun the four COBOL probes and add operation/operand regression coverage.

### VM-040 initial COBOL validation

All four selected COBOL programs passed on real Erlang and in fresh matrix
processes with positive sentinels (rows 390-393). Fifteen arithmetic immediate
executions passed, including negative operands and all five binary operations.
All 12 Oct and 26 Nib BEAM programs passed again, along with 93 backend tests
and all-target Clippy for iir-to-beam and lang-aot. No seven-column rerun is
claimed. Remaining COBOL BEAM rows require further probes after this PR merges;
only four of the 58 COBOL rows now declare BEAM, for 410 total declared cells.

## VM-040 COBOL control and rounding probe (after #14670)

PR #14670 merged as 55f803c1ee after all 46 checks succeeded or skipped.
Prioritize the next four COBOL cases: IF/ELSE, rounded division, PERFORM TIMES,
and COMPUTE precedence. They reuse the newly proven literal arithmetic while
exercising comparisons, loop state and rounding. Probe real Erlang before any
coverage promotion. Preserve hard errors and exact stdout. Commit a separate
contract before backend changes if a defect appears. Larger strings and host
input remain behind this bounded proof; retain VM-060b and VM-013 separately.

### VM-040 COBOL comparison literal contract

The IF probe rejects Int(3) in cmp_gt. Extend the existing checked Var/Int
arithmetic operand conversion to the six comparison operations. Keep all
operand ordering, 0/1 result conventions and unsupported-kind errors. Verify
literal/literal and both mixed forms for less, equal and greater inputs across
all six comparisons, then rerun the four COBOL cases before promotion.

### VM-040 COBOL counted-loop literal move contract

IF and rounded division pass after comparison support. PERFORM TIMES then
rejects mov Int(3), used to initialize its counter. Permit checked Var/Int
operands in mov through the same conversion helper; retain missing-source and
unsupported-kind errors. The executed counted loop must print 1, 2, 3 and stop.
Include positive and negative immediate moves in direct runtime coverage.

### VM-040 COBOL control and rounding validation

The four selected programs pass real Erlang and fresh matrix processes with
positive execution sentinels (rows 395-398 before the final main refresh).
All 54 direct comparison/move cases, 15 arithmetic cases, the earlier four
COBOL cases, 12 Oct and 26 Nib programs pass. All 93 backend tests and Clippy
for both affected packages with all targets pass. Eight COBOL rows now declare
BEAM (414 total declared cells); no complete seven-column rerun is claimed.
Reprioritize remaining COBOL proofs against other BEAM and host ABI work after
merge, preserving the separate ALGOL owner.
