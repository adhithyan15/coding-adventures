## 0.369.0 - 2026-10-05 - Runtime-real assignable name actuals

- Preserve runtime-real formatter provenance when a specialised real name
  formal reads an assignable real array-element actual, including direct
  forwarding through another name formal. Existing specialised writes still
  target caller storage; no dynamic thunk ABI is introduced.
