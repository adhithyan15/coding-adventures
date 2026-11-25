# coding_adventures_python_parser (Dart)

Python parser: the legacy minor-line API plus the LANG84 release
frontend, driven by 17 compiled `python*.grammar` grammars (6
legacy lines + 11 release tables in 3 deduplicated classes). The
Dart lane of the Rust `python-parser` crate — one language
definition per Python version (`code/grammars/python/`) powers
every implementation.

```dart
import 'package:coding_adventures_python_parser/python_parser.dart';

final ast = parsePython('x = 1\n', '3.12');
print(ast.ruleName);
// file
final rel = parsePythonAt('print(1 + 2)\n', '3.13.16', const FrontendBudgets());
print('${rel.release} ${rel.ast.ruleName}');
// 3.13.16 file
```

## API

- Legacy: `parsePython(source, [version])` /
  `tryParsePython(source, [version])` /
  `createPythonParser(source, [version])` with
  `resolvePythonParserVersion` (`''` selects the default
  `'3.12'`) and the recursion-depth guard (`maxRuleDepth` 190).
  Trees root at the grammar's start rule (`file`, except 3.8's
  `file_input`).
- Releases: `tokenizePythonAt(source, release, budgets)` /
  `parsePythonAt(source, release, budgets)` with exact,
  no-fallback resolution (`resolvePythonRelease`,
  `supportedPythonParserReleases`, `pythonGrammarClasses`,
  `releaseParserGrammar`), `FrontendBudgets` (source/token/depth
  limits with trusted clamping), and the 5-variant
  `PythonFrontendError` (unknown release with suggestions,
  missing pair, budget, lex, parse).

## Deliberate divergences from the reference

- Unknown legacy versions throw `ArgumentError` with the exact
  message instead of `Err(String)`; legacy parse failures
  propagate the structured engine errors.
- Depth tests run in-process instead of on rigged worker threads
  (Dart has no thread-stack tuning): 5000-deep input must fail
  cleanly, never `StackOverflowError`.
- Lex error messages carry the Dart engine's payload-blind text
  (`Unexpected character.`) instead of echoing the offending
  character; release, line, and column are identical.
- Control-character escaping in diagnostics matches Rust's
  `escape_debug` over its testified contract; exotic
  format/mark characters Rust also escapes pass through here
  (probed against `rustc`, no test pins them, same security
  contract).
- Byte/length measurements count lone surrogates as U+FFFD
  (unrepresentable upstream; `utf8.encode` would throw on them).
- CSV/file-header conformance checks run at authoring time (the
  emitter reads the same sources); the suite pins the emitted
  inventory and transcribes the CPython fixtures instead of
  reading them from disk.

## Validation

- **Unit** (`test/python_parser_test.dart`,
  `test/release_test.dart`): 6 lib tests, 10 release probes (the 2
  CSV-content probes and the subprocess-harness probe are covered
  equivalently in-process), and 3 CPython tokenize differentials
  (6 fixtures transcribed verbatim) plus 12 lane extras.
- **Property** (`test/properties_test.dart`): 12 seeded properties
  over the in-lane `pbt` package — closed vocabulary, root
  shapes, determinism, both exact depth boundaries (legacy 9/10,
  every release 21/22), both valid corpora, API agreement,
  unknown-version rejection, resolution exactness, budget laws,
  error hygiene.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`, every input run under all 6 legacy
  versions and all 11 releases; only engine errors and
  `PythonFrontendError` escape, successes root correctly and
  re-parse deterministically.
- **Coverage**: 100% lines and functions.
