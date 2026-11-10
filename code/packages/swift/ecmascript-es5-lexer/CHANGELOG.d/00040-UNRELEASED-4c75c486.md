## [Unreleased]

### Fixed
- Updated `_Grammar.swift` to match `code/grammars/ecmascript/es5.tokens` after
  #19032 added `\u2028\u2029` to the `WHITESPACE` and `LINE_COMMENT` patterns
  without regenerating the Swift copy (patched by the generator's escaping rules
  and checked against the grammar source, because no Swift toolchain was
  available; re-run `regen-embedded-grammars.sh` when convenient). `swift/grammar-tools`'
  `regen-embedded-grammars.sh --check` reported it as STALE in every `main` full
  build. Only those two embedded patterns change (`[ \t\r\n\v\f\u2028\u2029]+`
  and `\/\/[^\n\r\u2028\u2029]*`); the lexer's behaviour now matches the grammar
  (U+2028 and U+2029 are whitespace and end a line comment).
