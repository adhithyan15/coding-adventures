## 0.1.1

- Gate soundness: corpus comparison is type-strict (canonical JSON), so `true`
  is not `1`, `false` is not `0` and `1.0` is not `1`; suite rows have a closed
  key set; fault and control order is compared exactly; a model or suite the
  oracle cannot lower is reported (`level`), never counted as agreement;
  non-object rows are refused cleanly.
- Evaluator: subject calls inside `model_call` arguments are counted; `wraps` is
  counted in one dedicated pass that replays memoised model wrap counts and
  evaluates the pure subject arguments, so it no longer depends on memoisation,
  suite order or depth; model chains and long expressions are refused with
  named `limit` errors (model depth 64, expression depth 64), never a
  `RecursionError`.
- Cost (VER04r 4.2 as amended): worst-path costing, default 2,000, raised only
  by `max_ir_cost` in `corpus.profile` (new strict `profile` reader); per-suite
  `ir_cost`; total work and IR work bounds enforced across suites. The
  `--max-ir-cost` and `--cost-model` flags are removed.
- CLI: `emit-v2` exits 3 when suites are not-live or refused (`--allow-partial`
  to accept); `--out` may not be an input, a symbolic link or lie in the spec
  tree.
- Stream tags follow the registry (VER04i 2.6.1): tuple and record `64`, variant
  `65`.
