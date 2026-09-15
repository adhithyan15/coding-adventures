### Added — Software Layers Implementation (PR #5)
- `lexer` — hand-written + grammar-driven tokenizer (76 tests, 98%)
- `parser` — recursive descent + grammar-driven parser (54 tests, 99%)
- `virtual-machine` — general-purpose stack-based VM, 20 opcodes (99 tests, 96%)
- `bytecode-compiler` — AST to bytecode compiler (34 tests, 100%)
- `grammar-tools` — reads .tokens/.grammar files with EBNF (66 tests, 97%)
- `pipeline` — end-to-end orchestrator (40 tests, 100%)
- Grammar-driven lexer and parser that work with any language's grammar files
- `python.tokens` and `python.grammar` grammar definitions
- JIT compiler spec and shell package

