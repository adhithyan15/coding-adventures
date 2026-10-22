---
category: BUILD files & dependency management
---

# Editing a canonical .tokens or .grammar file means regenerating the Swift embedded grammars

Several Swift lexers and parsers compile a copy of their canonical grammar into a
checked-in `_Grammar.swift`. The list lives in
`code/packages/swift/grammar-tools/regen-embedded-grammars.sh` and covers
`toml.tokens`, `xml.tokens`, `algol60`, `dartmouth_basic` and the ES1/3/5
token files.

The `swift/grammar-tools` BUILD runs that script with `--check` after its own
tests, and fails on any drift.

#17722 (Forme TOML pipelines) edited `code/grammars/toml/toml.tokens` but left
`swift/toml-lexer/Sources/TOMLLexer/_Grammar.swift` alone. The PR's own CI
stayed green because its affected-package build never rebuilt
`swift/grammar-tools`. The stale copy then surfaced on a later, unrelated PR
(#17803), whose forced full build failed shard 14 with
`STALE: code/packages/swift/toml-lexer/...`. All of grammar-tools' 68 tests
had passed.

Fix: from the repository root, run
`bash code/packages/swift/grammar-tools/regen-embedded-grammars.sh` and commit
the regenerated file. Then run the same script with `--check`, and run
`swift test` in the affected Swift package.

Do it in the same PR as any edit to a grammar in that manifest. The PR build
will not catch the drift for you.
