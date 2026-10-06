## 0.376.0 - 2026-10-05 - Call-invariant runtime-real provenance

- Preserve runtime-real formatter provenance across direct value and statement
  calls for scalar slots that remain unaliased caller-frame locals.
- Keep captured and call-by-name-promoted shared storage conservative; no
  dynamic thunk or closure ABI is introduced.
