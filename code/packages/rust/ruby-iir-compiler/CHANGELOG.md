# Changelog

## Unreleased

- Lower exactly two positional integer arguments in a parenthesized `puts`
  call to a bounded Rust VM builtin, emitting two newline-terminated values
  atomically after both expressions execute. Validate direct-AST comma and
  statement/argument wrapper shapes before lowering.

- Lower an exact zero-argument `puts()` grammar call directly to a bounded
  Rust VM builtin that emits one newline; validate direct-AST delimiters by
  token kind and effective grammar type on both parenthesized call forms.

- Accept one-argument bare `puts expression` calls through the existing Ruby
  parser tree and direct IIR lowering, with the same integer bounds as `puts(...)`.
- Reject forged direct-AST `puts` callees with non-name token classifications
  in both supported call forms.
- Use the fallible Ruby parser constructor so malformed lexical input is
  returned as a compiler error.
- Reject numeric-looking strings and legacy leading-zero octal literals in the
  decimal-only pilot instead of silently compiling them as base-10 integers.

## 0.1.0

- Add a bounded Ruby source-to-IIR numeric compiler and native VM runner.
- Bound directly supplied AST token counts and text fields before lowering.
