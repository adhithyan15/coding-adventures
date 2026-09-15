---
category: Compiler / VM / language pipeline
---

# New language frontends MUST wrap `GrammarLexer` / `GrammarParser`, not hand-write their own

Every Twig/Lisp/whatever frontend in the repo (Python, Rust, etc.) is a thin shim that loads `code/grammars/<lang>.tokens` and `<lang>.grammar`. The wrapper pattern is the canonical approach — see `code/packages/rust/brainfuck/` for the reference. The standalone `lisp-lexer` / `lisp-parser` Rust crates are NOT a model — they predate the grammar-tools refactor. Hand-writing forks the grammar into a second implementation that drifts silently.
