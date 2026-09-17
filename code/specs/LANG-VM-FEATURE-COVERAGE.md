# LANG VM feature and backend coverage

Audit base: `cd73f3ad86` (2026-09-05); corpus counts updated for VM-047b, VM-057, VM-047c, VM-039b, VM-061, VM-042 and VM-040 (COBOL BEAM boolean/EVALUATE, then string ops/reference modification, then reference-modification MOVE/trap, then STRING SIZE/delimiter, then UNSTRING/delimiter, then INSPECT TALLYING/REPLACING, then pointer/overflow, then regions/self-move, then Dartmouth BASIC BEAM pure-string family, then Dartmouth BASIC BEAM numeric baseline / BEAM03 f64 lowering, then BEAM03 continuation neg(f64)/f64_pow, then VM-LOOP-24's probe-first general-arithmetic/control-flow and math-builtin promotions, then BEAM04's ets-backed float-array representation, then VM-041's Twig call_closure-liveness fix and probe-first sweep plus its match/union fusion follow-up, then BEAM06's str-typed array element representation, then BEAM07's BEAM host input for Dartmouth BASIC `INPUT`/FLOW-MATIC `READ-ITEM`, then BEAM08's `RND` fix closing VM-018 and the entire non-ALGOL BEAM matrix). This is an inventory of the implemented
frontend families and their executable proof boundaries, not a claim that the
historical languages or every backend are complete. Follow-up IDs live in the
[completion backlog](LANG-VM-NON-ALGOL-BACKLOG.md).

## Reading the evidence

The [driver](../packages/rust/lang-aot/src/lib.rs) wires ten `Language` variants.
The [unified corpus](../packages/rust/lang-aot/tests/lang_matrix.rs) has eight.
Its seven standard columns are NativeAOT, LLVM, WASM, JVM, CLR, VM and JIT.
Twig, Nib, Oct, FLOW-MATIC, COBOL-60 and (as of VM-042) Brainfuck rows
additionally declare BEAM, each only for the specific rows independently
proven on real `erl` — see each row's "Additional proof boundary" below for
the exact count. All counts below are source **declarations**, not counts of
executions on every host. External tools can be absent; actual runner
failures must fail, rather than turn into skips.

Frontend `backend_compat` validators and `backend_encode` byte generation are
useful checks, but neither establishes runtime behavior. The driver performs
shared lowering passes after frontend compilation, so a raw frontend validator
refusal also does not imply the complete driver refuses that feature.

| Frontend | Unified rows | Declared cells (all backends, Beam included) | Additional proof boundary |
|---|---:|---:|---|
| Twig | 49 | 392 | All 49 of those cells are BEAM (VM-041 + this follow-up: `match`/`union` fusion fix); dedicated heap/closure/string/union tests |
| Nib | 26 | 208 | All eight columns including real BEAM u4/u8 and BCD storage |
| Brainfuck | 6 | 45 | Dedicated WASM/JVM/CLR and JIT execution; 3 of 6 rows also real BEAM (VM-042) |
| Dartmouth BASIC | 51 | 408 | All 51 of those cells are BEAM (18 pure-string + 2 numeric-baseline + 2 neg/pow + 12 general-arithmetic/control-flow + 5 math builtins + 4 arrays/DATA + 2 string arrays + 5 `INPUT` rows + `RND`, BEAM03/VM-LOOP-24/BEAM04/BEAM06/BEAM07/BEAM08); random differential suite and frontend JIT tests |
| Oct | 12 | 96 | All eight columns, including real BEAM stdout and u8 wrap; frontend JIT control-flow tests |
| ALGOL 60 | 233 | 1631 | Separate owner; full-matrix CI exclusion remains VM-025; not re-audited by VM-061 (see below) |
| FLOW-MATIC | 8 | 64 | All eight rows now declare Beam (BEAM07 promoted the four `READ-ITEM`/EOF rows that were on seven columns) |
| COBOL-60 | 58 | 464 | All 58 of those cells are BEAM (VM-040 COBOL BEAM slices); much larger frontend JIT/oracle suite |
| McCarthy Lisp | 0 | 0 | Dedicated 19-program capstone with nine runner lanes |
| Macsyma | 0 | 0 | Dedicated 21-program capstone with eight runner lanes plus real CoreCLR |

The normal non-ALGOL capstone therefore declares 210 programs and 1677
declared cells (sum of the non-ALGOL rows above, updated for BEAM07's nine
newly-promoted cells — 5 Dartmouth BASIC `INPUT` rows + 4 FLOW-MATIC
`READ-ITEM`/EOF rows, each gaining one new Beam cell — and BEAM08's one
newly-promoted cell, Dartmouth BASIC's `RND` row). **BEAM08 closes the
LAST undeclared non-ALGOL BEAM cell in this entire table**: Dartmouth
BASIC now declares all 51/51 rows on `Beam`, and every non-ALGOL frontend
in this table now declares BEAM on every row it is ever going to (Twig
49/49, Nib 26/26, Oct 12/12, COBOL-60 58/58, FLOW-MATIC 8/8, Dartmouth
BASIC 51/51, and Brainfuck 3/6 — the other 3 need real stdin-as-tape host
support, a separate, still-unscoped item unrelated to `RND`/VM-018). See
`code/specs/BEAM08-rnd-beam-support.md` and
`LANG-VM-NON-ALGOL-BACKLOG.md`'s "BEAM08" section for the full research
and fix (issue #15332's `global_store`/`gc_bif2` bug was RND's sole
blocker — no new IIR op, no new BEAM opcode, and no frontend change was
needed). At VM-061 this matched a
fresh `non_algol_matrix_every_proven_cell_agrees` run exactly: 1338 cells
exercised plus 210 skipped (missing local `ilasm`) = 1548. VM-042 then added
three real Beam cells to Brainfuck (42 → 45 declared), so a fresh run on a
host with `erl` reported 1341 exercised + 210 skipped = 1551; VM-040's
boolean/EVALUATE slice added four more real Beam cells to COBOL-60
(422 → 426 declared), reporting 1345 exercised + 210 skipped = 1555; the
string ops/reference modification slice added four more real Beam cells to
COBOL-60 again (426 → 430 declared), reporting 1349 exercised + 210 skipped =
1559; the reference-modification MOVE/trap slice added four more real Beam
cells to COBOL-60 once more (430 → 434 declared), reporting 1353 exercised +
210 skipped = 1563; this slice (VM-040 COBOL BEAM STRING SIZE/delimiter) added
four more real Beam cells to COBOL-60 again (434 → 438 declared) — the first
COBOL BEAM cells to need `str_len`/`str_index` alongside `str_slice`/
`str_concat` — so a fresh run on a host with `erl` reported 1357 exercised
+ 210 skipped = 1567; this slice (VM-040 COBOL BEAM UNSTRING/delimiter) added
four more real Beam cells to COBOL-60 once more (438 → 442 declared) — one
`STRING ... DELIMITED BY delim` row and three `UNSTRING` rows, all reusing
`str_len`/`str_index`/`str_slice` unchanged from the prior slice, so no new
`iir-to-beam` lowering was needed — so a fresh run on a host with `erl` now
reports 1361 exercised + 210 skipped = 1571; this slice (VM-040 COBOL BEAM
INSPECT TALLYING/REPLACING) added four more real Beam cells to COBOL-60 once
more (442 → 446 declared) — the base `INSPECT TALLYING FOR ALL/CHARACTERS/
LEADING` and `REPLACING ALL` rows, all compiling through
`emit_inspect_tallying`/`emit_inspect_replacing`/`emit_inspect_region_window`,
which use only `str_len`/`str_index`/`cmp_*`/`const`/`add`/`sub`/`mov`/`jmp*`/
`label`/`and`/`or` — every one already lowered for BEAM before this slice, so
no new `iir-to-beam` lowering was needed here either — so a fresh run on a
host with `erl` now reports 1365 exercised + 210 skipped = 1575; this slice
(VM-040 COBOL BEAM pointer/overflow) added all eight remaining
pointer/overflow cells to COBOL-60 at once (446 → 454 declared) — the first
COBOL BEAM row to reach `emit_string_pointer_overlay`, the shared `STRING`/
`UNSTRING ... WITH POINTER` helper that chains three `str_slice` calls and
two `str_concat` calls with every bound computed entirely at run time from a
live `PIC 9` pointer item — so a fresh run on a host with `erl` now reports
1373 exercised + 210 skipped = 1583; no `iir-to-beam` defect was found, all
eight programs passed on the first real-`erl` probe. With all 58 COBOL-60
rows then declaring BEAM (VM-040 COBOL BEAM regions and self-move), a fresh
reprioritization audited every non-ALGOL frontend's row/BEAM-declared count
directly from source (not by memory) and found Dartmouth BASIC — despite
being explicitly in scope since VM-040's original family list ("BASIC
f64/I/O") — was the only non-ALGOL frontend with **zero** declared BEAM
rows out of 51. A `compile_source_to_beam` probe over all 51 rows found
`iir-to-beam` rejects every one of them at whole-module validation: BASIC's
`BA7-1b` change routes every scalar numeric value, even an integer-spelled
literal, through the shared `f64` value track, and the frontend
unconditionally emits its `__basic_print_real`/`__basic_print_fixed_mag`/
`__basic_print_real_e` helpers into every compiled module regardless of
whether a given program's control flow ever reaches them — so `iir-to-beam`'s
whole-module float-const check (it has no `f64` lowering support at all)
rejects every numeric BASIC program, even ones that never print a float.
Exactly 18 rows — every purely string-valued program, with no numeric
`PRINT`/`LET`/`FOR`/`INPUT` anywhere in source, so no float `const` is ever
emitted — compile unchanged and all 18 ran correctly on real `erl` with
byte-identical stdout (VM-040 Dartmouth BASIC BEAM pure-string family;
357 → 375 declared BASIC cells). The full `non_algol_matrix_every_proven_cell_agrees`
capstone confirmed the corrected total against a live run: 210 programs,
**1401** cells exercised, 210 skipped (the same host-wide missing `ilasm`
pattern every prior slice reports), zero failures, in 602.62s — and
1401 + 210 = 1611 matches the corrected declared total exactly.

That slice explicitly deferred BASIC's numeric corpus: `iir-to-beam` had no
`f64` lowering support of any kind (no `f64`/`Float`/`fadd`-family match arm
anywhere in `lower.rs`). BEAM03 (`code/specs/BEAM03-float-lowering.md`) adds
it — `const`(f64) via a new module literal table (`ir-to-beam` 0.4.0's `LitT`
chunk support, hand-rolled RFC 1950/1951 zlib encoder targeting this repo's
pinned OTP 27 CI runtime), `int_to_real`/`real_to_int_trunc` via new
single-argument `gc_bif1` BIF calls, and `add`/`sub`/`mul`/`cmp_*` needing NO
code changes at all (they already lower generically over any register
contents). This promotes the two BASIC "numeric baseline" rows immediately
preceding the pure-string family (375 → 377 declared BASIC cells): `10 PRINT
42` (BA7-1b's paradigm case, exercising `__basic_print_real`'s full op chain)
and `10 PRINT 6 ^ 2 + 6` (a literal-integer-exponent `^`, adding `mul`/`add`
beyond the bare-literal case). Both passed on real Erlang
(`portable_text_stdout_dartmouth_basic_beam_numeric_baseline`). A genuine
defect (VM-D034) was found and fixed along the way: the existing i64 `div`
lowering used `erlang:div/2`, which traps on a float operand; f64 `div` now
dispatches to `erlang:'/'/2`. General f64 division by zero remains a
documented, deliberately out-of-scope platform gap (real Erlang floats
cannot represent IEEE-754 Inf/NaN at all), not reachable by either promoted
row. No full `non_algol_matrix_every_proven_cell_agrees` capstone rerun is
claimed for this slice; the dedicated numeric-baseline test above and the
full `iir-to-beam`/`ir-to-beam` suites (including new real-`erl`
integration tests for the float op set) are the executed evidence. Dartmouth
BASIC now declares 20/51 rows on Beam; the remaining ~26 numeric/`FOR`/
`LET`/`RND` rows need `neg`(f64) and `f64_pow` (still unimplemented) plus
the unscoped BEAM host-input design (VM-060b) for the 5 `INPUT` rows.

A BEAM03 continuation slice then added both remaining candidates named
above. `neg`(f64) needed **zero** lowering changes: `iir-to-beam`'s existing
`"neg" | "not"` arm already dispatches unconditionally to `erlang:-/1`
(`gc_bif1`), which is already polymorphic over integer and float operands —
unlike `div` (VM-D034), Erlang's unary minus has exactly one operator, and
the only per-`type_hint` branch in that arm is a `u4`/`u8` narrowing mask
that `"f64"` never matches. `f64_pow` needed real design work: `math:pow/2`
is an ordinary Erlang function, not a loader-recognized guard BIF, so it
cannot use `gc_bif1`/`gc_bif2` the way `int_to_real`/`real_to_int_trunc` do;
it lowers to a `call_ext` instead, the same pattern `str_concat`'s
`erlang:'++'/2` and `str_index`'s `lists:nth/2` already use. This promotes
two more BASIC rows (377 → 379 declared BASIC cells): `10 PRINT ABS(-42)`
(ABS's inline `if X < 0 then -X else X` lowering, plus the unary-minus
literal `-42` itself, exercises `neg`(f64) twice) and `10 PRINT 4 ^ 0.5`
(a fractional exponent that misses the literal-integer-exponent fast path
and falls through to the general `f64_pow` runtime call). Both passed on
real Erlang (`portable_text_stdout_dartmouth_basic_beam_neg_and_pow`). No
new platform-limitation surface was found: `f64_pow`'s Inf/NaN gap is the
same already-documented §6 divergence `div` has, not exercised by the
promoted row (base `4`, exponent `0.5`, exact finite result `2.0`). Dartmouth
BASIC now declares 22/51 rows on Beam; the remaining ~29 numeric/`FOR`/`LET`/
`RND` rows still need the unscoped BEAM host-input design (VM-060b) for the
5 `INPUT` rows, plus whatever other ops the still-unpromoted `FOR`/`DEF FN`/
array/`DATA`/`RND`/`GOSUB` rows exercise beyond what BEAM03 has covered so
far (each needs its own probe before promotion, not an assumption that
`neg`/`f64_pow` alone unblocks them).

VM-LOOP-24 then ran exactly that per-row probe sweep the prior slice called
for, over every remaining non-`INPUT` BASIC row. 12 rows ran on real `erl`
completely unchanged (general `FOR`/`FOR … STEP`/`IF … THEN` control flow, a
same-module `DEF FN` call, multi-item `PRINT` with `;`/`,`, ordinary scalar/
fixed-decimal/significant-digit real formatting, flat and nested `GOSUB`/
`RETURN` via the existing `i64`-array return-address stack, and `SGN`) —
zero new `iir-to-beam` lowering (379 → 391 declared BASIC cells). A further
5 rows needed new but bounded/mechanical lowering: `SQR`/`SIN`/`COS`/`LOG`/
`EXP`/`ATN`/`TAN` each lower to a single-argument `math:*` `call_ext` (the
exact `f64_pow` shape, confirmed the same way via `erlc -S` disassembly, just
with one operand instead of two), and `INT` additionally needed
`real_to_int_floor`, which disassembles to the SAME `gc_bif1` guard-BIF shape
`int_to_real`/`real_to_int_trunc` already use (`erlang:floor/1`, unlike the
`math:*` functions above it) (391 → 396 declared BASIC cells). Three groups
remain explicitly deferred as genuine design questions, not guessed at: 1-D/
2-D numeric arrays plus `DATA`/`READ`/`RESTORE` (4 rows) compile but trap at
runtime — `iir-to-beam` represents every array with Erlang's `atomics`
module, which is INTEGER-ONLY, so any `array<f64>` write raises `badarg`;
fixing this needs a real BEAM float-array data-representation decision, not
an opcode addition. String arrays and mixed numeric/string `DATA` (2 rows)
fail validation outright — `iir-to-beam` has no `str`-typed array element
representation at all. `RND` (1 row) still traps with `{badarith,
[{erlang,'*',[undefined,...]}]}` inside its compiled helper — confirmed to be
the same "RND's full DEF-FN-and-module-global chain" open question flagged
at VM-018, not something `real_to_int_floor` unblocks as a side effect. See
`code/specs/BEAM03-float-lowering.md` §9 for the full probe transcript and
`LANG-VM-NON-ALGOL-BACKLOG.md`'s "VM-LOOP-24" section for the reprioritization
note. Dartmouth BASIC now declares 39/51 rows on Beam.

BEAM04 (`code/specs/BEAM04-float-array-representation.md`) then closed the
first of VM-LOOP-24's two deferred design questions: the 1-D/2-D numeric-
array and `DATA`/`READ`/`RESTORE` rows compiled cleanly but trapped at
runtime (`{badarg,[{atomics,put,[Ref,Index,FloatValue],...}]}`), because
`iir-to-beam` represented every `alloc_array`/`array_set`/`array_get` with
Erlang's `:atomics` module, which can only hold 64-bit integers. Two
directions were researched concretely against real `erl`/`erlc -S`, not
guessed at: (a) bit-reinterpreting the f64 as a 64-bit integer via BEAM
bit-syntax, reusing `:atomics` unchanged — the bit pattern round-trips
exactly for every finite f64 tested, but the actual BEAM instruction shape
needed (`bs_create_bin`/`bs_start_match4`/`bs_match`, confirmed by
disassembling a real compiled bit-syntax round-trip) is an entirely new,
materially more complex operand-encoding subsystem this backend has never
needed; (b) switching float-element arrays to `:ets`, which stores
arbitrary terms (floats included) natively — confirmed via `erlc -S` to
need **zero** new BEAM opcodes: `ets:new/2`/`ets:insert/2`/
`ets:lookup_element/3` are ordinary `call_ext`s (the same shape `math:*`
already uses), and the `{Idx, Val}` insert tuple is built with `put_list` +
`erlang:list_to_tuple/1` (the same list-construction op `call_closure`'s
arg-list already uses). (b) was chosen on that evidence — the backlog's own
prior guess that reusing `:atomics` would "likely" be simpler did not hold
up once actually measured. This promotes four more BASIC rows (396 → 400
declared BASIC cells): the 1-D array row, the 2-D array row, the `DATA`/
`READ`/`RESTORE` row, and the fractional-`DATA` row, all executed on real
Erlang (`portable_text_stdout_dartmouth_basic_beam_arrays_and_data`). A
known, documented limitation was found and left unfixed (not exercised by
any promoted row, since each writes every cell it later reads): unlike
`atomics:new`, `ets:new` does not pre-zero N cells, so reading a
never-`array_set` index traps (`badarg`) instead of returning `0.0`.
String-typed arrays remain a separate, larger gap (`iir-to-beam` has no
`str`-typed array element representation at all — independent of the
float-storage question just closed), and `RND` remains blocked on VM-018's
module-global design question, neither touched by this slice. Dartmouth
BASIC now declares 43/51 rows on Beam; the only remaining gaps are the 5
`INPUT` rows (VM-060b), the 2 string-array/mixed-`DATA` rows, and `RND`.

VM-041 then turned to Twig, the last non-ALGOL frontend with a large
undeclared BEAM surface (20/49 rows, the remaining 29 framed only as
unscoped "dynamic-string/record/closure BEAM isolation" design work). A
confirmed silent-data-corruption bug had to be fixed first: `iir-to-beam`'s
`"call_closure"` lowering emits TWO `call_ext` instructions
(`erlang:'++'/2` then `erlang:apply/3`), and neither was wrapped in the
liveness save/restore macros — the same VM-D029 bug class already fixed
once for the six `:atomics` ops, dormant here because no `lang_matrix.rs`
row exercising `call_closure` had ever declared `Beam`. With the fix landed
(and a real-`erl` regression test proving a variable now survives across a
`call_closure` call), a probe-first sweep of EVERY one of the 29 not-yet-
`Beam` Twig rows against real `erl` found **27 pass completely unchanged** —
dynamic `any`-typed arithmetic, the `length`/`list-ref`/`assoc` recursive
list-walk helpers, both closure rows (no-capture and capturing, unblocked
by the fix above), and all 19 string-op rows (`iir-to-beam`'s string ops
were already proven on BEAM for other frontends; these Twig rows had simply
never been individually probed). Only 2 rows (`match`/`union`) remain
deferred: a validator gap was found and fixed along the way (`"mov"` with a
`ref<LispyPair>` type_hint was rejected even though `lower.rs` already
lowers it correctly for any type — relaxed to mirror the existing `"str"`
exception, proven correct by a dedicated real-`erl` test), but that fix
alone is not sufficient — both rows hit a SEPARATE, deeper, still-open gap:
the `alloc`+`field_store`+`field_store` → `put_list` fusion only recognizes
the three instructions immediately adjacent, and the synthesized
union-variant constructor interleaves a `mov` between them. Twig now
declares 47/49 rows on Beam (363 → 390 declared cells); only tagged-union
pattern matching remains.

A direct follow-up then pinned down and closed that exact gap. Compiling the
`match`/`union` corpus row through `compile_source_to_iir` and dumping the
`Some` constructor's IIR (a scratch probe, discarded before the PR) showed
the interleaved instruction is a **`box`**, not the `mov` guessed at above —
and it sits between `alloc` and the FIRST `field_store`, not between the two
`field_store`s: `twig-ir-compiler::emit_union_def`'s per-field cons-cell loop
emits `alloc`, then `box` (E6d-6b: boxing the field for the tagged backends'
`match`/`unbox` round-trip), THEN the two `field_store`s. Of the two fix
directions the backlog left open — teaching `iir-to-beam`'s fusion
look-ahead to tolerate interleaving, or changing the union-variant
constructor's own codegen — the codegen fix was chosen: `box` has no data
dependency on the freshly allocated cell register (it only reads the field
value), so hoisting it to before the `alloc` changes no semantics on any
backend and merely restores the adjacency the fusion needs, exactly
mirroring this same function's tag/head cons cell a few lines below, which
already computed its own `box` before its `alloc`. This is strictly less
machinery than a general N-instruction-skip scanner in `iir-to-beam` (a
backend every other frontend also depends on) for a concrete case with
exactly one interleaved-instruction shape. Proven by
`twig-ir-compiler`'s new `union_constructor_alloc_immediately_followed_by_
its_two_field_stores` test (adjacency, direct on the emitted IIR) and
`lang-aot`'s new `twig_beam_match_union` test (full pipeline, real `erl`,
both promoted rows). Twig now declares **49/49** rows on Beam (390 → 392
declared cells) — fully complete. Combined with COBOL-60 (58/58) and Nib/Oct
(already full), Twig closing to 49/49 left Dartmouth BASIC's remaining 8
design-blocked rows (5 `INPUT` rows on VM-060b, 2 string-array/mixed-`DATA`
rows, `RND` on VM-018) as the ONLY non-ALGOL BEAM gap left in this backlog.

BEAM06 then took up the string-array/mixed-`DATA` gap — the one item of
those three that was NOT actually an unscoped design question, once
investigated. `iir-to-beam`'s validator rejected `array_set`/`array_get`
with `type_hint == "str"` outright (`UnsupportedType`), and BEAM04 had
explicitly framed this as needing "a BEAM representation for a
*heterogeneous* element type." Research tested that framing directly rather
than accepting it: `str_const`'s existing scalar lowering already
represents a `str` value as an ordinary Erlang character list, and BEAM04's
`:ets`-backed array substrate already stores arbitrary Erlang terms
natively — confirmed on real `erl` with a standalone probe mirroring
`iir-to-beam`'s exact `array_set`/`array_get` instruction shape (round-trip,
overwrite, concatenation of two round-tripped elements, and the
empty-string edge case all pass). Tracing `dartmouth-basic-iir-compiler`'s
mixed-`DATA` lowering directly also found the "heterogeneous element"
framing did not apply in practice: BASIC's `DATA` pool materializes THREE
separate parallel typed arrays (kind/numeric/string), never one array
holding mixed element types — so no tagged/variant representation was
needed anywhere. The fix was a pure widening of the existing `:ets`-dispatch
condition (`"array<f64>"`/`"f64"` → also `"array<str>"`/`"str"`) plus a
matching validator exception, with the emitted BEAM instructions
byte-for-byte identical to the existing float-array path. Full research:
`code/specs/BEAM06-string-array-representation.md`. Promoted both blocked
rows (`DIM A$(2)` and the mixed numeric/string `DATA`/`READ`/`RESTORE` row)
via a dedicated real-`erl` test. Dartmouth BASIC now declares **45/51** rows
on Beam (400 → 402 declared cells); the only remaining gaps (5 `INPUT` rows
on VM-060b, `RND` on VM-018) are genuinely unscoped design questions, not
probe-and-promote items — this closes out every non-ALGOL BEAM gap in this
backlog that does not require a new, out-of-scope architectural decision.

The "Declared cells" column counts every backend a
row proves, Beam included — the convention Nib, Oct, FLOW-MATIC and
COBOL-60's numbers already used. Twig was the one holdout at VM-061: its old
"343" was `49 rows × 7 standard backends`, silently excluding its 20 Beam
cells (VM-D030/**VM-061**).
Every other non-ALGOL row's declared row count and cell count were
independently re-derived from `PROGRAMS` in `lang_matrix.rs` (a `Prog { lang:
Language::X, .., backends: &[..] }` per row; cells = `rows.map(|p|
p.backends.len()).sum()`) via a brace-balanced parse of the whole array, not
a hand count or a quick regex, and all six matched the doc exactly except
Twig at that time. That derivation is pinned as
`feature_coverage_doc_counts_match_programs_source` in `lang_matrix.rs`: it
asserts each of these seven non-ALGOL row/cell pairs against the live
`PROGRAMS` corpus, plus that McCarthy Lisp and Macsyma still have zero rows
there (both use the dedicated capstone files below instead), so a future
slice that adds or removes a row without updating this table fails a normal
`cargo test -p lang-aot --test lang_matrix` run instead of drifting silently
again — VM-042's own Brainfuck change, and each COBOL BEAM slice's COBOL-60
change, updated both the test's expected tuple and this row together, exactly
as that test's own doc comment requires. ALGOL
60's row is intentionally left unrecomputed and unasserted: it is owned by a
separate, actively developing campaign (see "Ownership boundary" in the
completion backlog), and this table's own audit trail (VM-D030/VM-061) does
not extend a mandate to correct or pin its count. The zeroes for McCarthy and
Macsyma mean dedicated coverage, not absent support. CLR-real is an
additional runner lane for the same CLR backend in McCarthy's capstone, not a
tenth universal backend.

## Implemented feature families and remaining proofs

| Frontend/source | Implemented family and current executable evidence | Boundary and next work |
|---|---|---|
| [Twig lowerer](../packages/rust/twig-ir-compiler/src/compiler.rs) | Scalars, variadic arithmetic, lexical bindings, calls, cons/list operations, symbols, globals, records/unions, closures and source-inferred strings. Unified rows cover heap arithmetic, list helpers, quote equality, records/match, forward and boxed globals, capturing closures, literal/local/parameter strings and a bounds trap. | BEAM covers all 49/49 rows (VM-041's call_closure-liveness fix + probe-first sweep, then this follow-up's union-variant `alloc`/`box`/`field_store` reorder for the `match`/`union` fusion gap). Twig has no further BEAM gaps in this backlog. |
| [Nib lowerer](../packages/rust/nib-iir-compiler/src/lib.rs) | Integer arithmetic, narrow masking, wrapping/saturating addition, bitwise/logical operations, branches/loops, calls, const/static initialization and BCD storage. Unified rows execute standard-backend cases. [JIT tests](../packages/rust/nib-iir-compiler/tests/jit_e2e.rs) independently exercise compiled functions. | Standard-target parity does not establish 4004 arithmetic/control-flow fidelity. Existing VM-028 owns that audit; VM-012 proves only its landed BCD storage slice. BEAM remains undeclared (VM-040). |
| [Brainfuck compiler](../packages/rust/brainfuck-iir-compiler/src/compiler.rs) | All eight commands, wrapped tape cells/pointer movement, nested loops and input/EOF. Unified rows plus [WASM](../packages/rust/brainfuck-iir-compiler/tests/wasm_e2e.rs), [JVM](../packages/rust/brainfuck-iir-compiler/tests/jvm_e2e.rs), [CLR](../packages/rust/brainfuck-iir-compiler/tests/clr_e2e.rs) and [JIT](../packages/rust/brainfuck-iir-compiler/tests/jit_smoke.rs) execution. | VM-042/VM-D031: the frontend README's "BEAM tape support intentionally excluded" claim was stale — `iir-to-beam`'s `:atomics`-backed mutable memory (added for this exact purpose) already lowers tape mutation and `.`; a real `erl` probe promoted the 3 non-input rows to a real Beam cell each. `,` still refuses explicitly (no `getchar` builtin), pinned by `call_builtin_getchar_rejected_but_putchar_accepted`; that gap is host input (VM-060b), shared with every other frontend's BEAM input rows, not a Brainfuck- or tape-specific limitation. |
| [BASIC lowerer](../packages/rust/dartmouth-basic-iir-compiler/src/lib.rs) | f64 arithmetic/general power/transcendentals, deterministic RND, scalar/string input and output, branches, FOR, GOSUB/RETURN, DEF FN, numeric/string arrays, mixed DATA/READ/RESTORE. All 51 rows declare all eight columns, including BEAM (18 pure-string + 2 numeric-baseline + 2 neg/pow + 12 general-arithmetic/control-flow + 5 math builtins + 4 arrays/DATA + 2 string arrays + 5 `INPUT` rows + `RND`, BEAM03/VM-LOOP-24/BEAM04/BEAM06/BEAM07/BEAM08); random differential tests supplement fixed results. | DEF FN global access and historical print zones are frontend semantics, not already-implemented parity. No BEAM gaps remain: BEAM07 closed the 5 `INPUT` rows (VM-060b, BEAM host input); BEAM08 closed `RND` (VM-018) — its trap turned out to be caused entirely by `global_store`'s pre-existing `erlang:put/2`-via-`gc_bif2` bug (issue #15332), not a new design question, so fixing that bug was both necessary and sufficient. String-typed arrays and mixed numeric/string `DATA` (2 rows) were closed by BEAM06: the existing `:ets`-backed array substrate (BEAM04) already stores `str` values natively (a `str` value is already an ordinary Erlang character list), needing zero new representation work. Two-dimensional numeric DIM already has a seven-column matrix proof; the stale one-dimensional-only README wording is corrected in this audit. |
| [Oct lowerer](../packages/rust/oct-iir-compiler/src/lib.rs) | u8 arithmetic/masking, bitwise/logical operations, functions, local/global state, if/while/loop/break and stdout `out`. Matrix covers output, wrap, short circuit, shared globals, loop-carried wrapping returned from a function, conditional break and nested break targets; [JIT suite](../packages/rust/oct-iir-compiler/tests/jit_e2e.rs) separately executes while loops and returned function values. | VM-044 adds observable loop/break and returned-call standard-column proofs. `in`, carry arithmetic and rotations are explicit intrinsic errors; VM-013 owns portable machine-state design. Body-local static and floats are not implemented parity gaps. |
| [ALGOL lowerer](../packages/rust/algol-iir-compiler/src/lib.rs) | Scalar integer/boolean/real/string operations, arrays, procedures, by-name specializations, switches and nonlocal control flow have a substantial evolving corpus. [Frontend JIT](../packages/rust/algol-iir-compiler/tests/jit_e2e.rs) and [AOT smoke](../packages/rust/algol-iir-compiler/tests/aot_smoke.rs) are separate proofs. | Full LANG matrix remains excluded for the recorded native-array failure. VM-025 and the separate ALGOL owner control fixes and detailed feature expansion; 232 declarations do not mean 232 green Linux programs. |
| [FLOW-MATIC lowerer](../packages/rust/flow-matic-iir-compiler/src/lib.rs) | MOVE, COMPARE/IF/OTHERWISE, GO TO/JUMP, STOP, READ-ITEM/EOF and WRITE-ITEM. Four unified rows prove scalar move/output, a taken EQUAL, false LESS/GREATER reaching OTHERWISE, and a jump chain. [JIT stream tests](../packages/rust/flow-matic-iir-compiler/tests/jit_e2e.rs) run read/process/write to EOF through custom `input_more`/`input_i64` builtins. | VM-037 adds terminating, output-discriminating control-flow rows; positive LESS/GREATER on nonzero input still requires VM-039. VM-039 provides portable EOF-aware input before promoting record streams to code-generation columns. TRANSFER and tape control are clean frontend rejections, not secretly implemented file I/O. |
| [COBOL lowerer](../packages/rust/cobol-iir-compiler/src/lib.rs) | PICTURE/scaled arithmetic, DISPLAY/MOVE, condition names, IF/EVALUATE, PERFORM/GOTO, COMPUTE/power, size errors and signed/alphanumeric operations occur in the matrix, now expanded to 58 rows with ASCII reference modification: literal/computed bounds, comparisons, MOVE fitting and invalid-bound traps, plus STRING SIZE full-width copying, truncation and untouched tails, STRING delimiters and UNSTRING field fitting/empty fields/exhaustion, pointer/overflow branches, a self-referential `STRING` proof (VM-057), and INSPECT TALLYING/REPLACING BEFORE/AFTER region proofs including the not-found asymmetry and BEFORE+AFTER used together across one combined statement's independently-regioned halves (VM-047c). The [JIT/oracle suite](../packages/rust/cobol-iir-compiler/tests/jit_e2e.rs) additionally exercises reference modification, STRING, UNSTRING and INSPECT families. | VM-045 adds reference-modification rows; VM-046a/b/c add STRING SIZE, delimiter/splitting and pointer/overflow rows; VM-047a adds ALL/CHARACTERS/LEADING tallying proofs; VM-047b adds replacement including first-match/non-rechaining; VM-047c adds BEFORE/AFTER region proofs. A single delimiter phrase carrying BOTH `BEFORE` and `AFTER` together (the ISO two-delimiter intersection) parses but currently reads only the first region clause on both the oracle and the compiler (VM-D027); genuine intersection support is a separate follow-up. Validator acceptance is insufficient. Existing byte/character and category restrictions must remain explicit. |
| [McCarthy lowerer](../packages/rust/mccarthy-lisp-iir-compiler/src/lib.rs) | Quote/cons/CAR/CDR/ATOM/EQ/COND, direct and higher-order lambdas, captured variables, LABEL recursion and closure values. [Capstone](../packages/rust/lang-aot/tests/conformance.rs) tests 19 integer-result programs; [frontend run tests](../packages/rust/mccarthy-lisp-iir-compiler/tests/run_e2e.rs), [JIT](../packages/rust/lang-aot/tests/jit_mccarthy.rs) and dedicated per-backend lambda suites cover further shapes. | VM-036 enables the native capstone on Linux/macOS/Windows, with a required native-only Windows CI run. The capstone is not a proof for every closure shape; preserve dedicated closure suites in normal BUILD. |
| [Macsyma lowerer](../packages/rust/macsyma-iir-compiler/src/lower.rs) | v0 integers, unary/binary arithmetic, exact literal division, assignments, symbols and unevaluated symbolic Apply. [Oracle suite](../packages/rust/macsyma-iir-compiler/tests/oracle.rs) compares symbolic results with the evaluator. [Capstone](../packages/rust/lang-aot/tests/macsyma_conformance.rs) proves 21 integer-result programs on VM/JIT/WASM/CLR/JVM/LLVM/native when present; [`clr_real_macsyma.rs`](../packages/rust/lang-aot/tests/clr_real_macsyma.rs) additionally proves the same corpus on **real CoreCLR** (`dotnet`+`ilasm`), not only the in-repo CLR simulator (VM-049). | BEAM executes the same 21 integer programs when Erlang is present (VM-038); symbolic result representation has VM oracle coverage, not portable capstone agreement (VM-048). Function definitions/calls, control flow, floats, lists, comparisons and power are explicit frontend rejections, outside implemented v0 parity. |

## CI and host boundaries

[lang-aot/BUILD](../packages/rust/lang-aot/BUILD) explicitly includes the
`conformance`, `macsyma_conformance`, `jit_mccarthy`, per-backend lambda/heap,
and `cargo_archive_path` targets in its first cargo command. It separately runs:

```sh
cargo test -p lang-aot --test lang_matrix t7_differential_random_basic_
cargo test -p lang-aot --test lang_matrix portable_text_stdout_
cargo test -p lang-aot --test lang_matrix non_algol_matrix_every_proven_cell_agrees -- --exact --nocapture
```

The complete unfiltered `lang_matrix` target is excluded (VM-025). Frontend
BUILD files run their own package tests, including FLOW-MATIC's stream JIT and
COBOL's JIT/oracle suite. Such frontend executions do not establish LLVM or
native execution of the same source. The [CI workflow](../../.github/workflows/ci.yml)
uses affected-package planning; Windows additionally has a selected native
executable smoke gate. A green Windows job does not imply every LANG test
executes there. VM-036 adds the native-only McCarthy corpus to that actual Windows execution
command, including the required-linker assertion.

`lang-aot/BUILD` declares `# needs-toolchain: dotnet` (VM-049/VM-D028): its own
bucket language is "rust", so without this declaration the planner never set
CI's `needs_dotnet` flag for a PR touching only this crate, and
`actions/setup-dotnet` plus the `ilasm` NuGet restore — both gated on that
flag in `ci.yml` — never ran. The CLR-real tests (`clr_real_*.rs`,
`clr_real_macsyma.rs`) always skipped *correctly* without the toolchain, but
that meant the CLR-real column effectively never executed on its own PR
merge-gate CI, only on a forced main-branch full build (which sets every
toolchain flag). The declaration follows the exact pattern
`java-to-semantic-ir/BUILD` already uses for its own extra Python dependency.

Reproduce the dedicated capstones with:

```sh
cargo test -p lang-aot --test conformance --test macsyma_conformance --test clr_real_macsyma -- --nocapture
```

McCarthy always runs VM/JIT/WASM/CLR simulator. Java, clang, Erlang and real
CLR require their respective installed tools; native uses the host Linux/macOS/Windows compiler and linker.
Macsyma always runs VM/JIT/WASM/CLR simulator, gates JVM/LLVM/native on tools,
and now gates a BEAM runner on Erlang (VM-038). VM-049 added a real-CLR arithmetic proof
(`tests/clr_real_macsyma.rs`, gated on `dotnet`+`ilasm`, over the identical
21-program corpus the simulator column already agrees on) so simulator
success is no longer the only claim of CLR execution; the simulator column
itself is unchanged and remains the required always-on floor. VM-049 also
fixed VM-D028: `lang-aot/BUILD` lacked a `needs-toolchain: dotnet` declaration,
so no PR touching only this rust-bucketed crate ever made hosted CI install
`ilasm`, and the CLR-real column — including McCarthy's pre-existing one —
never actually executed on its own PR merge-gate CI, only on a forced
main-branch full build.

## Executed audit validation

At the audit base, the dedicated command above passed on Windows: McCarthy reported
19 programs across VM/JIT/WASM/CLR/JVM/LLVM/BEAM (133 agreements); native-AOT
was excluded by its macOS guard and CLR-real was unavailable. Macsyma reported
21 programs across VM/JIT/WASM/CLR/JVM/LLVM/native-AOT (147 agreements), plus
its process-result decoder test. These are actual local executions, not inferred
from declaration counts. Hosted CI remains the merge gate.

VM-036 locally executes the 19-program native corpus with both Microsoft and
LLVM Windows linkers. The required-linker negative invocation fails with zero
programs run when PATH is empty. Linux/macOS and hosted Windows execution are
still checked by PR CI before merge.

VM-037 locally executed all 21 newly declared FLOW-MATIC cells (three programs
on seven backends), each with a fresh-process execution sentinel and no skips.

VM-044 locally executed all 21 newly declared Oct cells, each in a fresh process
with its execution sentinel and no skips. The loop returns 24 after wrapping
250 + 30, conditional break prints 3, and nested breaks preserve output 4/2/7.

VM-045 locally executed all 49 new COBOL cells (rows 401–407), with zero skips
and fresh-process ran-cell sentinels. The run exposed and repaired native and
LLVM computed-slice refusals and WASM runtime-slice receiver output (VM-050–052).
The frontend oracle suite separately passed 66 reference-modification tests.

VM-046a locally executed all 21 new STRING SIZE cells (408–410) with zero
skips and ran-cell sentinels. Repeated writes exposed WASM's same-block
last-literal substitution (VM-053), repaired with runtime value propagation.
The frontend STRING/UNSTRING oracle selection separately passes 121 tests.

VM-046b locally executed all 35 new delimiter cells (411–415), with zero skips
and fresh-process sentinels. The run exposed native loop-index constant folding
and LLVM runtime-index refusal (VM-054/055); both now use checked runtime byte
indexing where needed. The frontend oracle selection passed 121 tests.

VM-046c locally executed all 56 pointer/overflow cells (416–423) in fresh
processes with ran-cell sentinels and zero skips. All 121 frontend
STRING/UNSTRING oracle tests passed; this slice needed no runtime change.


VM-047a locally executed all 21 tallying cells (424–426) in fresh processes
with positive sentinels and zero skips. The same three programs pass the
frontend oracle comparison; the full INSPECT-filtered suite passes 287 tests.

VM-047b locally executes all 35 replacement cells (427–431) with positive
sentinels and zero skips after the VM-056 WASM concat alias repair. All 292
INSPECT oracle tests and 236 WASM package tests including doctests pass.

The VM-047b/056 full local non-ALGOL run passed all 200 programs and 1420
cells, with zero skips (635.72 seconds).

VM-057 adds one seven-backend cell (row 432) for a self-referential COBOL
`STRING S DELIMITED BY SIZE INTO S`, after confirming and repairing a real
WASM `str_slice` destination-aliases-source defect. All 632 frontend
JIT/oracle tests and all 237 `iir-to-wasm` package tests including doctests
pass.

VM-047c locally executed all 30 available new cells (rows 433–437 × six
tool-present backends) in fresh processes via the single-cell re-verification
path with positive ran-cell sentinels and zero failures; CLR is an explicit
missing-tool skip on this host (`ilasm` not locatable), reproduced identically
on the pre-existing row 432. All 649 `cobol-iir-compiler` package tests pass,
including six new INSPECT BEFORE/AFTER and VM-D027 regressions. Investigating
BEFORE/AFTER used together on one delimiter phrase exposed VM-D027 — both the
oracle and the compiler silently honor only the first of two grammar-legal
region clauses — promoted to VM-058 rather than fixed in this slice. The full
non-ALGOL matrix passed 206 programs, 1256 cells exercised and 206 skipped
(every program's CLR cell, matching the host-wide missing `ilasm`), zero
failures, in 501.12 seconds.

VM-049 added `clr_real_macsyma.rs`'s toolchain-independent
`macsyma_emits_valid_cil_text_for_full_corpus` test, which locally compiles all
21 Macsyma programs to textual CIL and passes (no lowering change was needed:
`iir-builtin-lowering::dynamic_arith` already expands Macsyma's `call_builtin
"+"/"-"/"*"/"/ "` to `unbox`/`add`/`box` before `emit_il` runs). The real-CoreCLR
test itself reports the expected honest skip on this host (`dotnet`/`ilasm`
both absent, consistent with VM-047c). Investigating why the pre-existing
McCarthy `clr_real_*` lane's tool gate always reported a skip rather than a
real pass exposed VM-D028 — `lang-aot/BUILD` never declared
`needs-toolchain: dotnet`, so hosted CI's `ilasm` restore step never ran for a
PR touching only this crate; confirmed against a recent merged PR's Linux job
log (`needs_dotnet=false`). Fixed by adding the declaration, following the
same pattern `java-to-semantic-ir/BUILD` already uses. Hosted CI on this PR
is therefore the first actual proof (or disproof) that the CLR-real column —
Macsyma's new lane and McCarthy's pre-existing one — executes on real CoreCLR
rather than skipping.

VM-038: the full 21-program Macsyma corpus passed on real Erlang locally with
zero skips. Every source also compiles unconditionally in `macsyma_beam_corpus`.
Hosted execution is requested through BUILD's Elixir/setup-beam declaration;
this does not extend the scalar proof to symbolic values (VM-048).

VM-039a adds four FLOW-MATIC input/EOF rows (438–441), each declaring native
AOT and LLVM only. All eight cells passed locally; a direct production C test
also verifies stable non-consuming peeks. The corpus now has 442 programs,
including eight FLOW-MATIC programs; these rows add eight declared cells, not
28. WASM/JVM/CLR input adapters and the shared VM/JIT harness remain VM-039
follow-ups. Existing frontend callbacks already prove VM/JIT record streams.

VM-039b: the same four FLOW-MATIC input programs now pass on WASM. Current
main has one additional ALGOL row, so their current indices are 439–442 and
the complete corpus contains 443 programs. This adds four declared cells and
no programs. JVM/CLR and VM/JIT matrix adapters remain subsequent slices.

VM-039c JVM: all four input/EOF rows and five BASIC input regressions passed
on a real JVM with positive execution sentinels and zero skips. The shared
Java host regression also checks repeated peeks, mixed numeric/string reads
and I/O failure propagation. CLR and VM/JIT EOF matrix coverage remain pending.

VM-039c CLR: four input/EOF rows and five BASIC input rows passed on real
CoreCLR with execution sentinels. Direct IIR probes verify repeated peeks,
mixed string/numeric reads, 32/64-bit widths and numeric EOF. Encoded CIL
simulator input is not included; VM/JIT matrix EOF callbacks remain pending.

VM-039d: all four input/EOF sources passed across the seven standard columns
(28 executions, zero skips), plus ten BASIC VM/JIT input cells. A direct
compiled JIT peek proof passed with a callback counter and failing fallback.
The normal JIT matrix remains a tiered pipeline; it is not a claim that every
source entry runs compiled. Encoded CIL input remains VM-059.

VM-040 Oct: all twelve BEAM cells executed in fresh processes with positive
single-cell sentinels and no skips. The dedicated real-BEAM corpus separately
checks stdout and zero return values. Intel-8008 semantics remain VM-013.

The remaining base INSPECT BEAM probe adds four declared COBOL cells
(454 to 458). All four ran on real Erlang; this slice does not report a
new full-capstone run. Six COBOL rows still lack BEAM declarations.

The regions/self-move probe executes the final six COBOL BEAM corpus rows.
All 58 existing COBOL programs now declare eight backends (464 cells).
This is corpus coverage, not full language support; VM-058 remains open.
No new full-capstone or other seven-column rerun is reported here.
