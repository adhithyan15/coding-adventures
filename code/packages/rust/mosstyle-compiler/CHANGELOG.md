# Changelog — mosstyle-compiler

## [0.1.0] — 2026-05-11

### Added

- Initial implementation of the `.msl` (Mosaic Style Language) compiler.
- `TokenGrammar` and `ParserGrammar` embedded in `_grammar.rs` (no runtime file I/O).
- `tokenize()` — wraps `GrammarLexer`; resolves DIMENSION, HASH_COLOR, TOKEN_REF as
  custom token types via `type_name` (not new `TokenType` variants — they map to
  `TokenType::Name` with `type_name` metadata per the GrammarLexer contract).
- `parse_style()` — wraps `GrammarParser` with the embedded parser grammar.
- `analyze()` — converts the raw `GrammarASTNode` into typed `StyleDef` IR.
- `validate()` — checks part-name existence against the optional part-map JSON.
- `emit_css()` — produces scoped CSS using `.mos-{Component}-{part}` class names.
- `compile()` — convenience function running all stages.
- **Default dark token palette** — `$color-surface`, `$color-text-primary`, etc. resolve
  to hex literals at compile time (UI15 §1 palette).
- **State selectors**: `hover`→`:hover`, `pressed`→`:active`, `focused`→`:focus-visible`,
  `disabled`/`selected`/`editing`/`error` → class-name selectors.
- 17 unit tests covering tokenizer, CSS emission, token resolution, state blocks, and validation.

### Grammar

```
file          = style_def ;
style_def     = KEYWORD NAME LBRACE { part_def } RBRACE ;
part_def      = KEYWORD NAME LBRACE { part_item } RBRACE ;
part_item     = state_block | property_decl ;
state_block   = KEYWORD NAME LBRACE { property_decl } RBRACE ;
property_decl = NAME COLON style_value SEMICOLON ;
style_value   = TOKEN_REF | HASH_COLOR | DIMENSION | NUMBER | STRING | NAME ;
```

### Bug fixes

- `GrammarParser::new` takes `Vec<Token>` and `ParserGrammar` by value (not by reference).
  Fixed incorrect `&tokens` and `&grammar` borrows in `parse_style()`.
- `TokenType::Lbrace` → `TokenType::LBrace` (correct capitalisation in test assertion).
