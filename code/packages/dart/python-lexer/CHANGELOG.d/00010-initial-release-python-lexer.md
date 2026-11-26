## 0.1.0

- Initial Dart lane of the python-lexer identity: `tokenizePython`
  over 6 compiled legacy `python*.tokens` grammars
  (`lib/src/_grammar.dart`, generated with the grammar-tools
  compiler), with `resolvePythonVersion` (`''` selects the default
  `'3.12'`) and `supportedPythonVersions` mirroring the reference
  constants, plus the 11 LANG84 release tables
  (`tokenGrammarForRelease`, `pythonGrammarClass`,
  `supportedPythonReleases`, owned here for the future
  python-parser lane).
- Tests mirror the reference suite (all 9 tests) plus 6 lane
  extras, 10 seeded property tests over the in-lane `pbt` package,
  and a 2,000-iteration hostile fuzz campaign over the in-lane
  `fuzz` package (every input runs under all 6 legacy versions and
  all 11 release grammars); 100% line and function coverage.
- Engine fix (same branch, separate commit): Dart indentation mode
  now applies the reference's built-in whitespace rule — see the
  lexer CHANGELOG.
