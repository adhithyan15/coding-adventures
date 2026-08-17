# Changelog

## Unreleased

- fix: recognise the `shift_expr` precedence level. #11257 inserted a new
  `shift_expr` precedence level between `add_expr` and `mul_expr`
  (`add_expr -> shift_expr -> mul_expr -> bitwise_expr -> unary_expr`) to
  support `<<`/`>>`, but only updated the Rust `nib-lexer`/`nib-parser`/
  `nib-iir-compiler`/`nib-type-checker` consumers. This checker's
  `expression_rules` allow-list omitted `shift_expr`, so `expression_children`
  filtered both operands out of an enclosing `add_expr` -- `check_expr`'s
  `add_expr` branch never saw its required 2 operands and silently fell
  through to the generic single-child passthrough at the bottom of the
  function, inferring a type from only the left operand and never validating
  or reporting a mismatch against the right one (e.g. `let c: u4 = a + b;`
  with `a: u4` and `b: u8` type-checked without error). `shift_expr` is now
  in the rule table; its (always single, since this Lua lexer does not yet
  tokenize `<<`/`>>`) child transparently passes through the existing
  single-child recursion, exactly like `bitwise_expr`/`unary_expr` already
  do. Added a regression test for a mismatched plain two-operand `a + b`.
- tighten the package `BUILD` script so the final `luarocks make` runs with
  `--deps-mode=none` after sibling rocks are bootstrapped, matching the
  repository's clean-build CI validator

## 0.1.0

- add the first Lua Nib type checker package
- validate the convergence-wave Nib subset used by the local WASM lane
- return a typed AST wrapper keyed by node identity
