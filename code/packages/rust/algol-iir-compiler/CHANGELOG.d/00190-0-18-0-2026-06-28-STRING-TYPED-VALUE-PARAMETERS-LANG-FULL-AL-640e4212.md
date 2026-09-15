## 0.18.0 — 2026-06-28 — String-typed value parameters (LANG-FULL AL4-str-params)

ALGOL 60 typed procedures can now accept `string`-typed value parameters.
Previously `specifier_scalar_type` rejected `"string"` with an `Unsupported`
error; now it returns `Ok(ScalarType::String)`, unblocking the full
`value s; string s` spec syntax for string formals.

When `compile_procedure` binds a string parameter, its slot is immediately added
to `literal_string_slots` — the same set that makes a locally-assigned variable
printable.  This means `print(s)` inside the body lowers to `print_str s` with
no special string-parameter handling: the parameter is pre-seeded by the call
site's `str_const` (literal) or slot (named variable), which the E4 type system
already ensures is a known string value.

The call site in `emit_call_common` type-checks string actuals the same way as
integer/real parameters (`value.ty != *expected`); no new infrastructure is needed
because `str`-typed function parameters are already proven on all 7 backends via
Twig (TW4).

New unit tests verify: compilation succeeds, the body emits `print_str`, the IIR
parameter carries type `str`, the call site emits `call echo …`, a named string
variable can be passed, and an integer actual to a string parameter is a
`CompileError::Type`.

