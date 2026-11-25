# coding_adventures_fsharp_lexer (Dart)

F# lexer driven by 15 compiled `fsharp*.tokens` grammars, one per
supported version. The Dart lane of the Rust `fsharp-lexer` crate —
and the first versioned Dart lane: one language definition per F#
version (`code/grammars/fsharp/`) powers every implementation.

```dart
import 'package:coding_adventures_fsharp_lexer/fsharp_lexer.dart';

final tokens = tokenizeFsharp('let value = 1', '10');
print(tokens.map((t) => '${t.type}:${t.value}').join(' '));
// KEYWORD:let NAME:value EQUALS:= NUMBER:1 EOF:
```

## API

- `tokenizeFsharp(source, [version])` tokenizes in one shot; the
  final token is always `EOF`.
- `createFsharpLexer(source, [version])` returns the underlying
  `GrammarLexer` for the resolved version.
- `resolveFsharpVersion(version)` maps `''` to
  `defaultFsharpVersion` (`'10'`) and validates against
  `supportedFsharpVersions` (1.0 through 10, in reference order).

## Deliberate divergences from the reference

- Unknown versions throw `ArgumentError` carrying the reference's
  exact message instead of returning `Err(String)`: Dart has no
  string-error idiom, and a bad version string is a programming
  error at the call site.
- Tokenization failures propagate the structured engine
  `LexerError` instead of the reference's `"F# tokenization
  failed: ..."` string wrap (the `toml-lexer` precedent).

## Validation

- **Unit** (`test/fsharp_lexer_test.dart`): all 3 reference tests
  plus 7 lane extras (default selection, exact error message,
  factory agreement, per-version EOF/positions, hostile NUL).
- **Property** (`test/properties_test.dart`): 9 seeded properties
  over the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, positions, default agreement, valid-snippet parsing,
  factory agreement (all across all 15 versions), unknown-version
  rejection, version identity.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`, every input run under all 15 versions; only
  `LexerError` escapes, every success ends with `EOF` and re-lexes
  deterministically.
- **Coverage**: 100% lines and functions.
