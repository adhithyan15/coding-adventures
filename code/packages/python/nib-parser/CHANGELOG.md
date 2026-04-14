# Changelog — coding-adventures-nib-parser

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-12

### Added

- `src/nib_parser/parser.py` — Core module implementing `create_nib_parser`
  and `parse_nib`. Loads `nib.grammar` from the `code/grammars/` directory via
  the same 6-level relative path navigation used by `algol-parser`. Imports
  `tokenize_nib` from `nib-lexer`, `parse_parser_grammar` from `grammar-tools`,
  and `ASTNode`/`GrammarParser` from `lang-parser` (not `parser`, to avoid
  shadowing the Python stdlib `parser` module).

- `src/nib_parser/__init__.py` — Public API exports: `create_nib_parser`,
  `parse_nib`. Module-level docstring explains the package's purpose and
  provides a one-line usage example.

- `tests/test_nib_parser.py` — 35 test cases across 9 test classes:
  - `TestFactory` — factory function returns `GrammarParser`, produces valid AST
  - `TestTopLevel` — empty program, `const_decl`, `static_decl`, multiple decls
  - `TestFunctionDeclaration` — void, typed-return, single/two params, multiple fns
  - `TestStatements` — all six statement types: let, assign, return, for, if/else,
    multiple statements in one body
  - `TestExpressions` — all arithmetic/bitwise/logical operators, hex literals,
    bool literals, calls, nested parens, relational operators
  - `TestForLoop` — basic range, u4 var type, non-empty body, const upper bound,
    hex bounds
  - `TestStaticAndConst` — statics used in fn body, const referenced in fn,
    hex values, bool statics
  - `TestErrors` — missing semicolon, missing brace, fn without body,
    bare statement outside fn
  - `TestCompletePrograms` — 5 realistic multi-declaration programs exercising
    counter patterns, if/else branches, BCD nibble masks, call chains, and
    complex mixed expressions

- `pyproject.toml` — Package metadata with `hatchling` build backend, Python
  ≥ 3.12, dependencies on `directed-graph`, `grammar-tools`, `lang-parser`,
  `lexer`, `nib-lexer`, `state-machine`. Ruff lint rules include ANN
  (type annotation enforcement). Coverage threshold 80%.

- `README.md` — Knuth-style README explaining the Intel 4004 constraints,
  Nib's safety model, how the package fits the compiler pipeline stack,
  the AST structure with a concrete example, grammar rules table, expression
  precedence table, and a complete program example.

- `CHANGELOG.md` — This file.

### Design decisions recorded

- **`lang_parser` not `parser`**: Following the same workaround as `algol-parser`
  to avoid shadowing the Python stdlib `parser` module. The import is
  `from lang_parser import ASTNode, GrammarParser`.

- **Grammar path**: Six levels up from `__file__` to reach `code/grammars/nib.grammar`,
  matching the depth established by `algol-parser`.

- **Empty programs are valid**: `program = { top_decl }` accepts zero declarations.
  The grammar allows this; the semantic checker (a later pipeline stage)
  enforces that `main` exists. The parser does not enforce this.

- **No `lang-parser` in `pyproject.toml` name**: The PyPI package name is
  `coding-adventures-lang-parser` (with a hyphen), but the Python import name
  is `lang_parser` (with an underscore). This distinction is the standard
  Python packaging convention.
