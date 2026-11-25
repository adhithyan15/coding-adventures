## 0.2.0

- Slice 2 of the Dart lane: audit envelopes (`OperationDecision`,
  `OperationRecord`, `Operation<T>`, `AuditSink`/`NoopAuditSink`/
  `VecAuditSink`) and the async `Backend` seam (`OpenBackend`,
  `DenyAllBackend`, scripted `TestBackend`, Zone-scoped
  `runWithBackend`/`runWithBackendAsync`).
- Platform mappings: NUL-bearing paths rejected fail-closed (dart:io
  truncates at NUL, std::fs errors); live `writeEnv` always throws
  `UnsupportedError` (process env is read-only in Dart); `listDir`
  keeps every basename; TestBackend lock-free; `withMessage`/`takeRecords`
  copy/drain instead of move.
- Tests mirror the 4 audit + 6 backend reference tests, add 22 unit
  extras, 6 seeded properties (seeds 281-286), and a 2x500-script
  async differential campaign; package manifest now declares fs
  read/write/create/delete/list + env read (intentionally not
  RWS-clean — separation is enforced per caller manifest).
