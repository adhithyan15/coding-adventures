# Changelog — @coding-adventures/forme-errors

## 0.1.0 — 2026-05-15

Initial release. Implements the FM01 §6 kernel error model.

### Added

- `StageError` — typed error a stage throws; carries `code`, `message`,
  `inputPath`, `inputId`, `stageName`, `cause`, `recoverable`, `fields`.
  `toJson()` emits a stable structured shape for logs/telemetry.
- `CapabilityError` — `StageError` subclass with `code` locked to
  `"CAPABILITY_DENIED"` and `recoverable` forced `false`. Carries the
  offending `capability` string.
- `CancellationError` — Error subclass (deliberately **not** a
  `StageError` per FM01 §6.3) with optional `reason` field.
- `isCancellationError(value)` — cross-realm duck-typing predicate
  that survives Worker / VM-context boundaries where `instanceof`
  alone fails.
- `ERROR_CODES` — frozen vocabulary of the 10 kernel-blessed codes
  from FM01 §6.1: `PARSE_ERROR`, `PARSE_FRONTMATTER_INVALID`,
  `PARSE_NO_DOCUMENT`, `CAPABILITY_DENIED`, `CANCELLED`, `UNCAUGHT`,
  `TIMEOUT`, `IO_NOT_FOUND`, `IO_PERMISSION_DENIED`,
  `NETWORK_UNREACHABLE`.
- `KernelErrorCode` type alias — string-literal union of every
  `ERROR_CODES` value.

### Spec adherence

No deliberate divergences from FM01. The code list excludes the
open-prefix realms `TRANSFORM_*`, `COLLECT_*`, `RENDER_*`, `EMIT_*` —
those are reserved-namespace prefixes for stages to invent their own
specific codes under, not concrete kernel codes.
