## 0.154.0 — 2026-06-28 — ALGOL string procedure parameters on all 7 backends (LANG-FULL AL4-str-params)

Two new matrix `Prog` cells proving ALGOL 60 string-typed value parameters on all
7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `integer procedure echo(s); value s; string s; print(s); echo('HELLO')` → stdout `HELLO`
  (string literal as actual argument)
- `string msg; … msg := 'HI'; say(msg)` → stdout `HI`
  (named string variable as actual argument)

No new backend code — `str`-typed function parameters were already proven by Twig
(TW4). The only change is in `algol-iir-compiler` 0.18.0: `specifier_scalar_type`
now accepts `"string"` specifiers, and `compile_procedure` adds string parameter
slots to `literal_string_slots` so `print(s)` works inside the body.

838 total proven cells pass.

