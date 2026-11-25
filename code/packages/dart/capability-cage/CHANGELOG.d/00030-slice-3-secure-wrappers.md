## 0.3.0

- Slice 3 of the Dart lane (completes the port): `fs` secure
  wrappers (`readFile`/`writeFile`/`createFile`/`deleteFile`/
  `listDir`) and `env` wrappers (`readVar`/`writeVar`). Each checks
  the manifest first — denial throws `CapabilityViolationError`
  directly (no `io::Error` channel exists in Dart) — then delegates
  to the ambient backend.
- Tests mirror the 4 + 6 reference wrapper tests, add 5 unit extras
  (wrong-action denial, live temp-dir round trips, denial leaves no
  filesystem trace, check-then-platform ordering, NUL backstop) and
  a 2x500-script async differential campaign pinning verdicts,
  denial triples, and backend-log gating.
