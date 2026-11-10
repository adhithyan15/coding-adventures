## [Unreleased]

### Fixed
- Regenerated `_Grammar.swift` from `code/grammars/ecmascript/es3.tokens` after
  #19032 added `\u2028\u2029` to the `WHITESPACE` and `LINE_COMMENT` patterns
  without regenerating the Swift copy. `swift/grammar-tools`'
  `regen-embedded-grammars.sh --check` reported it as STALE in every `main` full
  build. Only those two embedded patterns change (`[ \t\r\n\v\f\u2028\u2029]+`
  and `\/\/[^\n\r\u2028\u2029]*`); the lexer's behaviour now matches the grammar
  (U+2028 and U+2029 are whitespace and end a line comment).
