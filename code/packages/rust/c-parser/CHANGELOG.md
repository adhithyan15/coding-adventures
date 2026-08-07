# Changelog

## 0.1.0 — C integer-core parser (SIR27)

- Grammar-driven parser over `code/grammars/c/c.grammar`, wrapping
  `parser::GrammarParser`.  `parse_c` / `try_parse_c` / `create_c_parser`,
  yielding a `GrammarASTNode` CST rooted at `translation_unit`.
- Covers the SIR27 subset: function definitions with typed (multi-word) params,
  local declarations with initialisers, `if`/`else`/`while`/`for`/`return`/
  compound statements, and the full C expression precedence cascade
  (assignment → || → && → | → ^ → & → == != → relational → shift → additive →
  multiplicative → unary/`(T)e` cast → postfix call → primary).
- A `(T)e` cast is disambiguated from a parenthesised expression by the type
  keyword after `(` (PEG ordered choice).
