# Changelog — coding-adventures-starlark-parser

## 0.1.0 — initial release

- Implement `starlark_parser.parse(source)` — tokenize with `starlark_lexer`
  and grammar-parse with `parser.GrammarParser`, returning the root ASTNode.
- Implement `starlark_parser.create_parser(source)` — returns a `GrammarParser`
  instance for manual control (trace mode, etc.).
- Implement `starlark_parser.get_grammar()` — returns the cached `ParserGrammar`.
- Grammar loaded from `code/grammars/starlark.grammar` via 6-level path traversal.
- Grammar cached after first load to avoid repeated file I/O.
- Root ASTNode has `rule_name == "file"` (first rule in starlark.grammar).
- Comprehensive busted test suite covering:
  - Module API surface
  - Simple assignments (`x = 1`, augmented assignments, tuple unpacking)
  - Function calls (`print("hello")`, BUILD-style `cc_library(...)`)
  - Load statements (`load("//rules.star", "symbol")`)
  - Function definitions with parameters, defaults, and multi-statement bodies
  - List literals (`[1, 2, 3]`, `[]`, nested access)
  - Dict literals (`{"key": "value"}`, `{}`)
  - If/elif/else statements
  - For loops (simple and tuple-unpacking)
  - BUILD file patterns (multi-line rules, multiple rules, load + rule)
  - Expressions (arithmetic, comparison, boolean, ternary, lambda, attribute access)
  - `create_parser` round-trip
  - Error handling for invalid input
