## Unreleased - arithmetic's figures after the adopter split (VER04r 16.22 D76)

- README and `tests/fixtures/data-eval-v1/README.md` stated 62 arithmetic suites and 2,654,925 cases (the 9-binder claim refused since CV2-F17). The adopter has split
  that claim into two of 8 binders (`_cin_0`, `_cin_1`), so the oracle derives `arithmetic.warrant` whole again: **64 suites, 2,655,437 cases** (measured with
  `corpus_v2.derive`, 38 s; largest suite 284,258 cases and largest `ir_cost` 1,524 unchanged). `check --spec` reports 64 suites lowered and none refused. No source change (the oracle's nine-binder test, `test_emit_reports_a_nine_binder_claim_as_refused_limit`, uses an inline document and never read `arithmetic.warrant`).
- Two census pins that counted arithmetic's suites move from 63 to 64 (`tests/test_lower.py` `CENSUS`, `tests/test_cli_emit.py::test_census_counts_levels`).
