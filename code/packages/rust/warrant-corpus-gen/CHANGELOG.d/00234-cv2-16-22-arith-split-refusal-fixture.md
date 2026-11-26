## Unreleased - the nine-binder refusal tests no longer use `arithmetic.warrant` (VER04r 16.22 D76)

- `tests/fixed_lowering.rs` (`the_binary_refuses_a_document_beyond_the_evaluated_levels_and_writes_nothing`) and `tests/check_cli.rs`
  (`exit_status_table_success_drift_refusal_usage`) used `code/specs/arithmetic.warrant` as the document refused for "a claim has more than 8 binders"
  (VER04r 5.3, R75). The arithmetic adopter split that claim into two of 8 binders, so `arithmetic.warrant` now generates (empty profile: 66 files, `--check` clean) and
  the tests would have gone red. They use the new fixture `tests/fixtures/nine_binders.warrant` (one claim, nine `Bool` binders) instead. The intent is unchanged: write and
  `--check` both exit 2, the message names the 8-binder bound and nothing is written. No source change.
- `tests/census.rs`: arithmetic's pins (generator and oracle) go from 63 to 64 `ok` claims and the 11-document total from 364 to 365: the split adds one claim, none is refused.
