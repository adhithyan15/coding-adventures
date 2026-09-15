---
category: Repo policy / workflow reminders
---

# grammar-tools codegen for `-compiler` packages

(not `-lexer`/`-parser`): `generate-rust-compiled-grammars` only auto-discovers `*-lexer` and `*-parser` packages. For `*-compiler` packages that embed both token + parser grammars in one `_grammar.rs`, run separately: `grammar-tools -f compile-tokens <file>.tokens` and `grammar-tools compile-grammar <file>.grammar`, then combine the two function bodies under one header with both import groups. The `-f` flag is needed for `.tokens` files that use `escapes: standard` (the validator rejects it, but the compiler correctly emits `escapes: Some("standard")` in the struct).
