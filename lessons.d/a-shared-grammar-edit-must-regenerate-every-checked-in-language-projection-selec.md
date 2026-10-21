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

**Swift is a separate lane, and pull-request CI does not run it.** The same
toml.tokens change (#17722) and the ECMAScript token fix (#17901) both merged
with stale Swift copies (`code/packages/swift/*/Sources/*/_Grammar.swift`):

- `code/scripts/generate-compiled-grammars.sh` does not produce the Swift
  files. They come from
  `code/packages/swift/grammar-tools/regen-embedded-grammars.sh`, whose
  manifest lists which Swift package embeds which grammar.
- Its `--check` runs only in `swift/grammar-tools`' BUILD, and a PR that
  edits only `code/grammars/` does not make that package affected. The
  staleness first surfaced in a full 15-shard build (shard 14,
  `swift/grammar-tools` FAILED with every test passing and a `STALE:` line).

After a shared grammar edit, also run
`bash code/packages/swift/grammar-tools/regen-embedded-grammars.sh` and
commit what it changes. Do not run `generate-compiled-grammars.sh` with no
`--lane` to "catch everything": it rewrites ~50 unrelated files, including
`rust/sql-*/_grammar.rs`, which is hand-edited and must never be regenerated.
