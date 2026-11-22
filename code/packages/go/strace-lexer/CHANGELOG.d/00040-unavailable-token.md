## [Unreleased]

### Added

- `UNAVAILABLE` token (`<unavailable>`, before `ERR_LT`) in
  `code/grammars/strace/strace.tokens`: the result text strace prints when it
  could not fetch a result because the tracee died (`= ? <unavailable>`).
  `grammar_data.go` regenerated (KILN04 D35).
