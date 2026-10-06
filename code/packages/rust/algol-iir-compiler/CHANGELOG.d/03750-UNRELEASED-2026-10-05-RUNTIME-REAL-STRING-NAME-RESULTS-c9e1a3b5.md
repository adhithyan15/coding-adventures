## 0.375.0 - 2026-10-05 - Runtime-real string name-formal results

- Preserve formatter provenance for real procedure results whose string name
  formals are finitely specialised at direct call sites. Procedure calls with
  string-literal actuals are no longer mistaken for literal expressions.
- Keep unproven real name actuals conservative; no dynamic thunk ABI is added.
