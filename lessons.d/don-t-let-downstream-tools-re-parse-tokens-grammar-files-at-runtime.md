---
category: Repo policy / workflow reminders
---

# Don't let downstream tools re-parse `.tokens` / `.grammar` files at runtime

Those files are build-time-only artifacts: `<lang>-lexer/build.rs` and `<lang>-parser/build.rs` already compile them into Rust source via `grammar_tools::compiler`, baking the parsed `TokenGrammar` / `ParserGrammar` into the lexer/parser rlibs as struct literals. Tools that need keyword lists, brackets, or grammar rules should pull from those compiled artifacts (e.g. `twig_token_grammar_spec()`, `twig_grammar()`) — not re-parse the source files. The `<lang>-spec-dump` binary in each language's parser crate is the canonical exit point: it serialises the embedded grammars to a `LanguageSpec` JSON document for editor tooling. Same source of truth, no drift.
