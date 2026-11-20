## [Unreleased]

### Added

- `tail` accepts `UNAVAILABLE`, and `resumed_line` accepts the interrupted resume
  `<... NAME resumed> <unfinished ...>) = ?` (`RESUMED inner UNFINISHED RPAREN
  tail`), in `code/grammars/strace/strace.grammar`. The consumer pins the exact
  spelling; the grammar stays looser. `grammar_data.go` regenerated and the
  goldens refreshed (a refused line's furthest-failure message changed; no
  accepted line changed). `parser/testdata/strace.grammar` re-copied (KILN04 D35).
