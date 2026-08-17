# Changelog

## 0.1.2

- fix the same class of crash for shift (`shift_expr`) expressions: #11257
  inserted a new `shift_expr` precedence level between `add_expr` and
  `mul_expr` (`add_expr → shift_expr → mul_expr → bitwise_expr → unary_expr`)
  to support `<<`/`>>`, but only updated the Rust `nib-parser`/`nib-iir-compiler`
  consumers. This formatter's `EXPRESSION_RULES` set and `printExpression`
  switch — which read the shared `nib.grammar` file at runtime rather than a
  generated copy — didn't know about the new rule, so every `add_expr` (even
  plain `a + b`) now wrapped an unrecognised `shift_expr` node and threw
  `Malformed add_expr: expected at least one operand`. Added `shift_expr`
  alongside `add_expr`/`mul_expr`/`bitwise_expr` in both places.

## 0.1.1

- fix a latent crash on multiplicative (`mul_expr`) expressions: the Nib
  precedence cascade is `add_expr → mul_expr → bitwise_expr → unary_expr`,
  but the formatter's expression dispatch (`EXPRESSION_RULES` set and the
  `printExpression` switch) never listed `mul_expr`. Any program containing
  a `*`, `/`, `*%`, or `/?` operator parsed to an `add_expr` whose only
  operand was an unrecognised `mul_expr` node, so the printer threw
  `Malformed add_expr: expected at least one operand`. The `mul_expr` rule
  was added to the grammar in #5677 (N1, `*` and `/`); this wires the
  formatter to match. Surfaced when an unrelated `grammar-tools` change
  pulled `nib-formatter` back into the affected-package test set (its tests
  only re-run when a transitive dependency changes).
- add regression tests for `a * b` and the mixed `a + b * c` precedence
  chain so the dispatch gap cannot silently reopen.

## 0.1.0

- add the initial `@coding-adventures/nib-formatter` TypeScript package
- lower Nib parser ASTs into `format-doc` documents using shared formatter templates
- expose both `Doc`-level and end-to-end ASCII formatting entry points
- cover ugly-input normalization, wrapping, and idempotence with unit tests
- preserve line comments and blank lines through the source-based formatter path
- recover EOF comments by formatting from a trivia-rich parsed document rather
  than from the AST alone
- cover top-level, block, `else`, trailing, and EOF comment behavior with unit
  tests
