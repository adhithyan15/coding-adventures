---
category: Compiler / VM / language pipeline
---

# A shared grammar edit must regenerate every checked-in language projection selected by CI

Changing `code/grammars/toml/toml.tokens` and regenerating only the TypeScript
lexer left Ruby's checked-in `_grammar.rb` projection stale. The focused
TypeScript tests and builds passed, but the independent Ruby regeneration gate
failed. After any shared `.tokens` or `.grammar` edit, run every affected
checked-in generation lane (not merely the language being implemented) and
verify that rerunning each CI-selected lane leaves the worktree clean.

Swift's copies are their own lane, and PR CI does not run it: see
`a-shared-tokens-edit-must-regenerate-the-swift-embedded-grammars-too-and.md`.
Do not reach for `code/scripts/generate-compiled-grammars.sh` with no
`--lane` to "catch everything". It does not produce the Swift files, and it
rewrites ~50 unrelated files, including `rust/sql-*/_grammar.rs`, which is
hand-edited and must never be regenerated.
