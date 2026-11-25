## 0.3.0 - fuzz engine security hardening

### Fixed
- CWE-400: total minimisation work is bounded campaign-wide (at most
  `max_iterations` executions across all crashes); `FuzzOptions.minimize_budget`
  has a ceiling (`MAX_MINIMIZE_BUDGET`, 100,000). Crashes found after the total
  is spent are recorded with `CrashRecord.minimized == False`.
- `minimize_crashing_input(budget=...)` is always finite (default 2,000).
- `Harness` caches only in-scope code objects, capped at `MAX_CODE_CACHE`.
- Nested `Harness` instances no longer claim each other's timeouts (private
  budget exception per instance).
- A failed `Harness.__enter__` releases its `sys.monitoring` tool id.
- `format_tap_failure` raises `ValueError` unless `failure_id` is one of the five
  registered `FUZZ_FAILURE_IDS`.
- `FuzzReport.seed` is excluded from `repr` (seeds are never public).

### Documented
- VER06d: hang-protection limits, tracer suspension, global `restart_events`,
  memory worst case.
