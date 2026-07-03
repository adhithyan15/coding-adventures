# Changelog — moslayout-compiler

## [Unreleased]

### Added — U29-G3: `expr` non-terminal in prop values

- Ten new tokens in `moslayout.tokens` for the expression grammar:
  `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`, `.`.
  Maximal-munch ordering lists each two-character operator before any
  single-character prefix it shares (e.g. `<=` before `<`).
- Seven new productions in `moslayout.grammar`:
  `expr`, `or_expr`, `and_expr`, `eq_expr`, `rel_expr`, `unary`,
  `postfix`, `primary`. Operator precedence (low→high):
  `||`, `&&`, `== !=`, `< <= > >=`, prefix `!`, postfix `.`/`[]`.
- `prop_value` is now an alias for `expr`. `primary` still includes the
  four legacy forms (slot ref, NAME, NUMBER, STRING) so the chain-rule
  descent collapses cleanly for the no-operator case.
- New `LayoutPropValue::Expr(String)` variant carrying the reconstructed
  source substring (tokens joined with spaces). Backends parse it
  themselves until a future PR lowers it to a typed expression AST.
- `validate_for_node` and `validate_if_node` now accept `SlotRef` OR
  `Expr` for their `each:` / `when:` props. Error messages updated to
  document both shapes.
- Twelve new tests cover comparison, logical-AND, NOT, field access,
  index access, parenthesised, and nested expressions plus regression
  guards that `slot: x`, bare NAME, NUMBER, and STRING values still
  come back in their legacy variants — never as `Expr`.
- Per UI29 §3.3, arithmetic (`+`/`-`/`*`/`/`), string concatenation,
  ternary, and function/method calls are deliberately excluded.
- Grammar version stays at `1`.

### Added — STRING token + string-literal prop values

- New `STRING` token in `moslayout.tokens`: a double-quoted string
  literal with standard `\`-escapes (`\n`, `\t`, `\r`, `\"`, `\\`, `\0`).
- `prop_value` grammar rule grows a fourth alternative for `STRING`,
  alongside the existing slot/emit binding, keyword, and number forms.
- New `LayoutPropValue::String(String)` enum variant on `LayoutProp`.
  The lexer strips surrounding double quotes; the compiler resolves
  standard escapes so downstream emitters receive the literal text
  the author meant.
- Three new unit tests cover the basic literal, escape resolution,
  and the empty-string case.
- Resolves limitation #3 in `code/programs/typescript/visicalc/README.md` —
  `placeholder: "Enter formula"` is now expressible at the source level.

## [0.1.0] — 2026-05-11

### Added

- Initial implementation of the `.mll` (Mosaic Layout Language) compiler.
- `TokenGrammar` and `ParserGrammar` embedded in `_grammar.rs` (no runtime file I/O).
- `tokenize()` — wraps `GrammarLexer` with the embedded token grammar.
- `parse_layout()` — wraps `GrammarParser` with the embedded parser grammar.
- `analyze()` — converts the raw `GrammarASTNode` into typed `LayoutDef` IR.
- `validate()` — checks part-name uniqueness, slot/emit references, and single-root invariant.
- `emit_part_map_json()` — produces the part-map JSON consumed by `mosstyle-compiler`.
- `compile()` — convenience function that runs all stages in sequence.
- **Grammar**: `prop = NAME COLON prop_value | KEYWORD COLON NAME` — the shorthand form
  `slot: label` (without a named prop key) is supported as sugar for single-slot leaf nodes.
- 19 unit tests covering tokenizer, parser, analyzer, validator, and the full `compile()` path.

### Grammar

```
file       = layout_def ;
layout_def = KEYWORD NAME LBRACE { node } RBRACE ;
node       = NAME [ part_name ] [ LPAREN prop_list RPAREN ] [ LBRACE { node } RBRACE ] ;
part_name  = LBRACKET NAME RBRACKET ;
prop_list  = prop { COMMA prop } ;
prop       = NAME COLON prop_value | KEYWORD COLON NAME ;
prop_value = KEYWORD COLON NAME | NAME | NUMBER ;
```
