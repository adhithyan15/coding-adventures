## 0.1.0

- Initial Dart lane of the python-parser identity: the legacy
  minor-line API (`parsePython` / `tryParsePython` /
  `createPythonParser` over 6 compiled `python*.grammar`
  grammars, `maxRuleDepth` 190) plus the LANG84 release
  frontend (`tokenizePythonAt` / `parsePythonAt` with
  `FrontendBudgets`, the 5-variant `PythonFrontendError`,
  exact release resolution over the 657-row inventory, and 11
  release tables in 3 deduplicated classes).
  `lib/src/_grammar.dart` is generated with the grammar-tools
  compiler.
- Tests mirror the reference suites (6 lib tests, 10 release
  probes, 3 CPython tokenize differentials over transcribed
  fixtures; the 2 CSV-content probes and the subprocess-harness
  probe are covered equivalently in-process) plus 12 lane extras,
  12 seeded property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line and function coverage.
