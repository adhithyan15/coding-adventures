## 0.17.0 — 2026-06-28 — `sqrt` standard function (LANG-FULL AL8-sqrt)

`sqrt(E)` — the ALGOL 60 §3.2.4 hardware square root — is now recognised by
`algol-iir-compiler` and lowered to the new `f64_sqrt` IIR op.  The op carries
a `ScalarType::Real` result type, exactly like `real_to_int_floor` carries its
result type, so every backend's typed-dispatch path fires without change.

The lowering is gated on the presence of exactly one `real` argument; a non-real
or wrong-arity call is a compile-time `CompileError::Type`.  The proof program
`begin real r; integer result; r := sqrt(49.0); result := entier(r) end` exits
7 on all 7 backends.

