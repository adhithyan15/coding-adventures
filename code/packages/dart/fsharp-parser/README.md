# coding_adventures_fsharp_parser (Dart)

F# parser driven by 15 compiled `fsharp*.grammar` grammars, one per
supported version. The Dart lane of the Rust `fsharp-parser` crate —
and the first versioned Dart parser lane: one language definition
per F# version (`code/grammars/fsharp/`) powers every
implementation.

```dart
import 'package:coding_adventures_fsharp_parser/fsharp_parser.dart';

final ast = parseFsharp('let value = 1', '10');
print(ast.ruleName);
// compilation_unit
```

## API

- `parseFsharp(source, [version])` tokenizes with the Dart
  fsharp-lexer lane and parses into an `ASTNode` rooted at
  `compilation_unit`.
- `tryParseFsharp(source, [version])` is the nullable twin: `null`
  instead of throwing (mirrors the reference's
  `Result`-returning `parse_fsharp` with idiomatic Dart
  nullability).
- `createFsharpParser(source, [version])` returns the underlying
  `GrammarParser` with the recursion-depth guard (`maxRuleDepth`
  200) enabled.
- `resolveFsharpParserVersion(version)` maps `''` to the default
  `'10'` and validates against `supportedFsharpParserVersions`
  (1.0 through 10, in reference order).

## Deliberate divergences from the reference

- Unknown versions throw `ArgumentError` with the reference's
  exact message instead of `Err(String)`.
- Lex failures throw the engine's `LexerError` and syntax failures
  throw `GrammarParseError` instead of the reference's `"F# parse
  failed: ..."` string wrap.
- The depth test runs in-process instead of on a rigged worker
  thread (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean error — never a `StackOverflowError` —
  which proves the guard fires first.

## Validation

- **Unit** (`test/fsharp_parser_test.dart`): all 5 reference tests
  plus 6 lane extras (default selection, exact error message,
  factory agreement, factory rejection, per-version root shapes,
  syntax failures).
- **Property** (`test/properties_test.dart`): 9 seeded properties
  over the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, the exact paren boundary (27 levels parse, 28
  trip), valid-program parsing, API agreement (all across all 15
  versions), unknown-version rejection, version identity, and
  lexer/parser version parity.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`, every input run under all 15 versions; only
  engine errors escape, every success roots at `compilation_unit`
  and re-parses deterministically.
- **Coverage**: 100% lines and functions.
