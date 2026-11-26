## 0.1.0

- Initial Dart lane of the perl-lexer identity: `tokenizePerl`
  (unversioned LANG81 grammar, 64KiB source cap) plus the LANG84
  release tables (`perlReleaseTokenGrammar`, `perlGrammarClass`,
  `supportedPerlReleases` — 118 releases collapsing to 5 distinct
  class tables, mirroring the reference exactly).
  `lib/src/_grammar.dart` is generated with the grammar-tools
  compiler; the emitter asserts every release's fresh parse equals
  its class table modulo line numbers.
- Tests mirror the reference suite (all 5 tests) plus 5 lane
  extras, 9 seeded property tests over the in-lane `pbt` package,
  and a 2,000-iteration hostile fuzz campaign over the in-lane
  `fuzz` package (every input runs under the unversioned grammar
  and all 5 class tables); 100% line and function coverage.
