# Changelog

## Unreleased

- Use the fallible Ruby parser constructor so malformed lexical input is
  returned as a compiler error.

## 0.1.0

- Add a bounded Ruby source-to-IIR numeric compiler and native VM runner.
- Bound directly supplied AST token counts and text fields before lowering.
