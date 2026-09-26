### Security -- `$` in an expression string literal is no longer Dart interpolation (#15464)

`moslayout-compiler` re-quotes the string tokens inside an `Expr` but does not
escape `$`, and Dart expands `$name` and `${...}` inside string literals. So an
authored `If ( when: ( x == "${boom()}" ) )` put live Dart into the generated
file, getting around the grammar's limit of names, literals and operators.

- **Fix:** `from_pipeline` (and so `from_pipeline_with_options`) now works on
  a copy of the layout in which every `Expr` has each `$` inside a quoted
  string rewritten to `\$` (`escape_interpolation_in_expr`). Doing it once at
  the entry point covers every sink, including sinks added later.
- **Unchanged output:** text outside string literals is copied byte for byte,
  and an `Expr` with no `$` is not touched, so existing output is identical.
- **Why not the shared compiler:** `\$` is an invalid escape in Swift, so the
  escape is backend-specific.
- **Tests:** scanner cases and end-to-end `If` emission for `"${boom()}"` and
  `"$y"`, plus a positive control. The end-to-end test fails with the rewrite
  disabled. Checked with `dart run` that the emitted `"\${boom()}"` and
  `"\$y"` are literal strings and `boom()` is never called.

