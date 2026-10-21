# Changelog

## 0.2.0 — 2026-10-20

### Added — LANG84 phase 7a release API
- `parse_python_at(source, release, &FrontendBudgets)` and
  `tokenize_python_at` run the generic `GrammarLexer`/`GrammarParser` over
  exactly one tagged release's `python<release>.tokens`/`.grammar`.
  `resolve_release` maps `None` to `DEFAULT_RELEASE` (`"3.13.16"`, the host
  `python3`). It never falls back: an unknown ID is `UnknownRelease` with up
  to five prefix suggestions (escaped and truncated before echo), a row
  without a pair is `ReleaseNotAvailable`, and `""` and minor lines such as
  `"3.12"` are unknown releases.
- `PythonFrontendError` with the LANG84 C6 parser categories (`Lex`, `Parse`,
  `Budget`, `UnknownRelease`, `ReleaseNotAvailable`), each naming the release
  and a 1-based position.
- C5 budgets: 64 KiB of source (trusted ceiling 1 MiB), 16,384 tokens checked
  before parsing (trusted ceiling 262,144), and the crate's measured
  190-level parser depth cap, which a caller cannot raise.
- `supported_releases`, `grammar_classes`, `release_parser_grammar` for
  LANG84 C8 contract tests.
- `tests/release_probes.rs` (stage P): the inventory, tables and pair headers
  agree with `pair-evidence.csv`; release selection; error spans;
  py-int-print positive and negative probes for every paired release,
  including PEP 515 `_` (a lexer error in 3.0a1 only); per-release operator
  sets; and hostile probes in a child process.
- `tests/tokenize_differential.rs`: each host release's token table produces
  exactly the token stream that CPython 3.11.17, 3.12.3 and 3.13.16's
  `tokenize` recorded for six fixtures (`tests/fixtures/tokenize_oracle.py`).
  3.16.0a1 lexes like 3.13.16. 3.0a1 differs only where its tokenizer did.

### Changed
- `_grammar.rs` is generated as a LANG84 release family. The minor-line
  grammars moved to `_grammar::legacy_versions`. `parse_python` and
  `create_python_parser` are unchanged.

- Added the initial Rust Python parser crate backed by compiled grammars.
