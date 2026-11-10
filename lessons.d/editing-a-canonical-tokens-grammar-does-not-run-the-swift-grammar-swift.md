---
category: CI & GitHub Actions
---

# Editing a canonical .tokens grammar does not run the Swift _Grammar.swift drift guard on the pull request, so main goes red

#19032 added `  ` to the `WHITESPACE` and `LINE_COMMENT` patterns in
`code/grammars/ecmascript/es{1,3,5}.tokens` and was green. Every `main` full build
afterwards failed `swift/grammar-tools` with:

```
STALE: code/packages/swift/ecmascript-es1-lexer/Sources/EcmascriptES1Lexer/_Grammar.swift (regenerate with regen-embedded-grammars.sh)
```

The committed `_Grammar.swift` files are generated copies of the grammars. The
guard that notices drift, `regen-embedded-grammars.sh --check`, lives in the BUILD
of `swift/grammar-tools` only. A pull request that edits `code/grammars/**` does
not make `swift/grammar-tools` an affected package, so the guard did not run on
the PR; it first ran in the next forced full build, hours later, on `main`. The
same thing had already happened once before (#17901, fixed by regenerating).

What to do differently:

- When you change a file under `code/grammars/`, regenerate **every** committed
  consumer of it before you push, not just the ones whose tests you ran. The
  Swift copies are produced by `code/packages/swift/grammar-tools/regen-embedded-grammars.sh`
  (needs Swift); `--check` is the verification.
- A drift guard only protects you if the thing it guards is an *input* of the
  package that runs it. Make the grammar files inputs of the guard's package, so a
  grammar edit schedules the guard on the PR.
- With no Swift toolchain, the output can still be reproduced exactly: the
  generator parses the grammar and writes each definition's pattern as a Swift
  string literal (`\` becomes `\\`, `"` becomes `\"`, control characters become
  `\u{..}`), keeping `\uXXXX` in a regex as literal text and recording the source
  `lineNumber`. A checker that re-derives every definition from its source line
  and is first proven on files that are already fresh is a sound way to verify a
  hand patch.
