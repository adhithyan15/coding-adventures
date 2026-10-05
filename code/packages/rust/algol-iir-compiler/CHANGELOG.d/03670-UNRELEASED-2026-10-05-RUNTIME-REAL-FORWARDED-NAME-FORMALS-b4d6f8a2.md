## 0.367.0 - 2026-10-05 - Runtime-real forwarded name formals

Direct forwarding from one real call-by-name formal to another now preserves
the original non-assignable actual expression before classifying runtime-real
formatter provenance. The nested specialised sibling can therefore format the
value without a runtime thunk ABI.

Assignable actuals remain outside the proof.
