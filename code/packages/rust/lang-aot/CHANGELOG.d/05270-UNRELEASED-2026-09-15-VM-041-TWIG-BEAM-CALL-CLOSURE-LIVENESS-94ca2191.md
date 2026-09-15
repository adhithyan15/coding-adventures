## Unreleased — 2026-09-15 — VM-041 Twig BEAM: call_closure liveness fix, 27 rows promoted

Twig was 20/49 rows declared `Beam`, with the remaining 29 needing (per the
prior rung-4 audit) unscoped "dynamic-string/record/closure BEAM isolation"
design work (VM-041). Before attempting any promotion, a confirmed bug had
to be fixed first: `iir-to-beam`'s `"call_closure"` lowering emits TWO
`OP_CALL_EXT` instructions (`erlang:'++'/2` then `erlang:apply/3`), and
neither was wrapped in `save_live_across_imported_call!`/
`restore_live_across_imported_call!` — the same silent-data-corruption bug
class as VM-D029 (fixed once already for six `:atomics` ops). This was
DORMANT (no `lang_matrix.rs` row exercising `call_closure` declared `Beam`
yet), which is exactly why it had to be fixed before promoting any Twig
closure row. See `iir-to-beam`'s CHANGELOG (0.14.0) for the full fix and its
dedicated real-`erl` regression test
(`test_98_real_erl_call_closure_survives_live_across_call`, confirmed to
fail with the wrong value when the fix is reverted).

With the fix landed, a probe-first sweep (mirroring VM-LOOP-24's successful
pattern) compiled and ran EVERY one of the 29 not-yet-`Beam` Twig rows
individually against real `erl`, using ONLY pre-existing `iir-to-beam`
capabilities plus the fix above — no other new lowering. **27 of 29 passed
unchanged**, promoted via two new dedicated `lang_matrix.rs` tests, each
executed against real `erl` before promotion:

- `twig_beam_dynamic_arith_list_ops_and_closures` (8 rows): dynamic
  `any`-typed arithmetic over a `car`'d cons cell, the `length`/`list-ref`/
  `assoc` synthesized recursive list-walk helpers, and — thanks to the fix
  above — both closure rows (a no-capture closure and a capturing closure).
- `twig_beam_string_ops` (19 rows): string literals, `let`/`let*` locals,
  non-escaping top-level `define`s, `substring`, comparison operators
  (`string<?`/`string>?`), the documented `string-ref` out-of-bounds trap,
  and a top-level function whose `str`-typed parameter is inferred four
  different ways. All of `iir-to-beam`'s string ops (`str_const`/`str_len`/
  `str_index`/`str_concat`/`str_slice`/`str_cmp`) were already proven on
  BEAM for other frontends — these 19 Twig rows simply had never been probed
  individually before.

**2 of 29 (`match`/`union`) remain explicitly deferred**, not forced. While
probing, `iir-to-beam`'s validator was found to reject `"mov"` with a
`ref<LispyPair>` type_hint outright, even though `lower.rs` already lowers
`mov` correctly for ANY type_hint — relaxing that (mirroring the existing
`"str"`-type_hint exception, see `iir-to-beam` 0.14.0) is a real, tested fix
but is NOT by itself sufficient: both `match`/`union` rows hit a SEPARATE,
deeper error once validation passes — `alloc`+`field_store`+`field_store`
fusion (`put_list` construction) only recognizes the three instructions
immediately adjacent, and the synthesized union-variant constructor function
interleaves a `mov` between them (`field_store: found outside of
alloc+field_store+field_store pattern`). That is a genuine, still-open
lowering gap — the fusion look-ahead would need to tolerate (or skip over)
intervening non-field_store instructions — left deferred with this exact
description rather than guessed at.

`lang-aot` 0.341.0 → 0.342.0: Twig now declares 47/49 rows on `Beam` (up
from 20/49). `feature_coverage_doc_counts_match_programs_source` updated
(Twig tuple `(49, 363)` → `(49, 390)`, 27 new `Beam` cells, one per promoted
row); `LANG-VM-FEATURE-COVERAGE.md` updated to match.

Reprioritize after this merges: the `match`/`union` fusion-adjacency gap
just described is the only remaining Twig BEAM gap, and is now a much more
precisely scoped follow-up than the prior "dynamic-string/record/closure
isolation" framing — strings, records, dynamic arithmetic, list ops, and
closures are ALL now proven on Twig BEAM; only tagged-union pattern matching
remains.
