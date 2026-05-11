# Changelog — moslayout-compiler

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
