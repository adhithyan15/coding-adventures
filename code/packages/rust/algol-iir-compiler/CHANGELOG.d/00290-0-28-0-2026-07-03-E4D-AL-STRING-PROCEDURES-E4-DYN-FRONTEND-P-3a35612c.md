## 0.28.0 — 2026-07-03 — E4d-AL: string procedures (E4-dyn frontend payoff)

The first E4-dyn *frontend* payoff, now that all seven backends carry a runtime
string: **`string procedure`s**. A typed procedure whose return type is `string`
was previously rejected (`Unsupported("string procedures")`); it now compiles.

- **`procedure_parts`**: dropped the `ret == String` early rejection.
- **`compile_procedure`**: a string result slot (the procedure name) is seeded
  with `str_const ""` (a real empty-string handle) instead of `const 0`, and
  marked literal-backed so an unassigned path still returns a printable value.
  The body assigns the result like any string variable; a branch-selected
  assignment (`if n > 0 then p := 'HI' else p := 'LO'`) makes the result a
  genuinely runtime, control-flow-selected string.
- **`try_emit_standard_output_stmt`**: added a general string-expression path so
  `print(pick(1))` evaluates a string-procedure *call result* to a runtime handle
  and prints it via `print_str`. `print(42)` is now rejected by type (a clearer
  message) rather than by the old literal-only shape check.

Unit test `string_procedure_returns_runtime_string_and_prints` asserts the IIR:
`pick` is a function returning `str` whose result is assigned by `str_const` in
both branches and returned as `str`, and `main` calls it and prints the runtime
result. A `lang-aot` matrix cell proves it end-to-end on NativeAot + VM/JIT (the
columns that already carry a runtime string arriving as a call result). The
LLVM/WASM/JVM/CLR columns take their runtime-string path only for promoted-slot
operands today, so a string *return value* on those backends is the E4d-2b/3b +
JVM/CLR follow-up that will extend the cell to all seven.

