# Changelog

## Unreleased

- Accept one-argument bare `puts expression` calls through the existing Ruby
  parser tree and direct IIR lowering, with the same integer bounds as `puts(...)`.
- Use the fallible Ruby parser constructor so malformed lexical input is
  returned as a compiler error.
- Reject numeric-looking strings and legacy leading-zero octal literals in the
  decimal-only pilot instead of silently compiling them as base-10 integers.

## 0.1.0

- Add a bounded Ruby source-to-IIR numeric compiler and native VM runner.
- Bound directly supplied AST token counts and text fields before lowering.
