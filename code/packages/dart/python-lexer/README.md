# coding_adventures_python_lexer (Dart)

Python lexer driven by 17 compiled `python*.tokens` grammars: 6
legacy minor lines plus the 11 LANG84 release tables. The Dart lane
of the Rust `python-lexer` crate — one language definition per
Python version (`code/grammars/python/`) powers every
implementation.

```dart
import 'package:coding_adventures_python_lexer/python_lexer.dart';

final tokens = tokenizePython('x = 1\n', '3.12');
print(tokens.map((t) => '${t.type}:${t.value}').join(' '));
// NAME:x EQUALS:= INT:1 NEWLINE:\n EOF:
```

## API

- `tokenizePython(source, [version])` tokenizes in one shot; the
  final token is always `EOF`.
- `resolvePythonVersion(version)` maps `''` to
  `defaultPythonVersion` (`'3.12'`) and validates against
  `supportedPythonVersions` (2.7 through 3.12, in historical
  order).
- Release tables (owned here for the future python-parser lane):
  `tokenGrammarForRelease(release)` (exact ID or `null` — no
  default, no fallback), `pythonGrammarClass(release)` (opaque
  class id, shared by identical token files), and
  `supportedPythonReleases` (inventory order, not
  version-sorted, on purpose). Release IDs and minor lines never
  cross (LANG84 C4).

## Deliberate divergences from the reference

- There is no `createPythonLexer` factory: every Python grammar
  runs in indentation mode, which the engine serves only through
  the `grammarTokenize` one-shot (`GrammarLexer` is
  standard-mode only). The Dart toml-lexer lane makes the same cut.
- Unknown versions throw `ArgumentError` with the reference's
  exact message instead of `Err(String)`.
- Tokenization failures propagate the structured engine
  `LexerError` instead of the reference's string wrap.

## Validation

- **Unit** (`test/python_lexer_test.dart`): all 9 reference tests
  plus 6 lane extras (default selection, exact error message,
  determinism, unknown-release nulls, per-version EOF/positions).
- **Property** (`test/properties_test.dart`): 10 seeded properties
  over the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, positions, default agreement, valid-snippet parsing
  (all across all 6 legacy versions), release-table loading,
  unknown-version rejection, table separation, class stability.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`, every input run under all 6 legacy versions
  and all 11 release grammars; only `LexerError` escapes, every
  success ends with `EOF` and re-lexes deterministically.
- **Coverage**: 100% lines and functions.
