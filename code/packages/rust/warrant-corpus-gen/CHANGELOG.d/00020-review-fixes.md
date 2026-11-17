# 0.1.0 review fixes (CV2-S3a)

- v0 rule 3 now follows `model_call`: a model whose call closure has fixed-width
  `add/sub/mul/neg`, called with a `subject_op` in an argument, is `W0701` in a v0 document.
- Examples must be one subject call compared with a pure value; others are `W0701`; the
  pure-guard rule runs on examples.
- Cost: checked addition, and a limit above 65,536 is refused (as the oracle does).
- `fixed_neg` no longer overflows at `i128::MIN`; `check_pure_guards` and the view refuse
  trees deeper than 64 instead of recursing.
- A negative suffixed literal other than the signed minimum is a `type` error.
- Parity view and oracle dump script now cover op-row `admit`; snapshots regenerated; the
  oracle's `BUILD` runs the fixture `--check`.
- VER04r 16.4 records the divergences found.

## Known gaps (left for S3b / S4 part 2)

A claim with no subject call lowers to an always-passing suite (S3b should refuse);
classification re-types the subject side without context; the oracle admits IR depth 65;
profile whitespace and empty-token handling differ from the oracle.
