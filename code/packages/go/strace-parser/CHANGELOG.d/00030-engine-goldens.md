## Unreleased

### Added

- `TestGoldenStrace` (`golden_test.go`, `testdata/golden/strace.golden`): every
  line of the corpus, the refused corpus and the `.strace` fixtures, parsed
  under the default limits and five tight depth, memo and token limits, must
  reproduce the exact tree (rules, positions, nil-ness, tokens, shared
  `Children` structure) or error (kind, text, token) that the shared parser
  engine produced before its flat-array rewrite. Keyed by the hash of the input
  line, so corpus additions do not shift other cases.
