## 0.4.0 — integration

- Audit hardening (00080) merged with the 0.3.x evidence APIs (`Suite::run_recorded`,
  `main_recorded`, `manifest`, `warrant-src-v1` framing); both behaviours are kept.
- Consumers (ct-compare, warrant-grammar, warrant-ir, ca-assurance-report) updated
  for `#[must_use]` on `Verdict` and the new `FuzzReport`/`BenchmarkResult` fields.
