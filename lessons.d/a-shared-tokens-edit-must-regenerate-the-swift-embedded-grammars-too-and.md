---
category: CI & GitHub Actions
---

# A shared .tokens edit must regenerate the Swift embedded grammars too, and they can be regenerated without a Swift toolchain

**What went wrong.** #17722 edited `code/grammars/toml/toml.tokens` and #17901
edited `code/grammars/ecmascript/es{1,3,5}.tokens`. Both regenerated the
TypeScript and Ruby copies but not the Swift `_Grammar.swift` files that
`code/packages/swift/grammar-tools/regen-embedded-grammars.sh` maintains. Those
PRs never built a Swift package, so they merged green. Every later PR that
did build one failed `swift/grammar-tools` with
`STALE: …/_Grammar.swift (regenerate with regen-embedded-grammars.sh)`, and
all eleven Swift lexer/parser packages were then DEP-SKIPped.

**Fix.** Run `regen-embedded-grammars.sh` (or its `--check`) whenever a
`.tokens`/`.grammar` file in its manifest changes. If no Swift toolchain is
available (download.swift.org can be blocked), port the generator instead:
- `grammar-tools-embed` is ~130 lines plus `GrammarCompiler.swift`'s
  string-literal emitter.
- The Python `grammar_tools.token_grammar.parse_token_grammar` gives the same
  definitions once you apply Swift's nil rules: empty skip/reserved/context/
  layout lists and empty groups become `nil`, and `caseSensitive` is `nil`
  unless it is false.

Prove the port before trusting it:
1. It reproduces every current manifest file byte for byte.
2. It reproduces the stale file from the grammar's previous revision.

Then the regenerated file is the generator's output, not a hand edit.
