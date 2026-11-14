## 0.1.1

- **Bounded work.** A suite's domain product must equal its declared
  `cases + discarded` (checked at load and again before running), a suite must
  admit at least one case, and per-suite (`MAX_SUITE_PRODUCT`), total
  (`MAX_TOTAL_WORK`, including control re-runs) and binder-count caps apply.
- **Model IR.** A static cost bound (`MAX_IR_COST`) refuses model-call bombs at
  compile time; very long model chains and arithmetic operands outside `U64`
  (checked before the operation) are corpus errors.
- **Honest labels.** `domain-labels.json` sidecar labels each suite `full`,
  `bounded-set` or `sample` from its domain descriptors (backlog LP-012);
  written by `run --out`, enforced by `check` and `validate-manifest`.
- **Exact exception classes.** `errors=` rows match `type(e) is cls`.
- **Malformed corpus structure** of any field type is a `CorpusError` (exit 2),
  never a `TypeError`.
- Fixed-width ops must state their width in the IR; widths stated by a
  signature are asserted per call.
- `sabotage` children always get the runner's own directory first on
  `PYTHONPATH`, so replacing `PYTHONPATH` can no longer hide the runner.
