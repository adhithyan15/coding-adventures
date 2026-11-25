# coding_adventures_derive_parser (Dart)

Derive parser driven by the compiled `derive.grammar` grammar. The
Dart lane of the Rust `derive-parser` crate — one language definition
(`code/grammars/derive/derive.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_derive_parser/derive_parser.dart';

final ast = parseDerive('DIF(SIN(x), x)\n');
print(ast.ruleName);
// program
```

## API

- `parseDerive(source)` tokenizes with the Dart derive-lexer lane
  and parses into an `ASTNode` rooted at `program`.
- `tryParseDerive(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_parse_derive` with idiomatic Dart nullability).
- `createDeriveParser(source)` returns the underlying
  `GrammarParser` with the Derive grammar and the recursion-depth
  guard (`maxRuleDepth` 200) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures
  throw `GrammarParseError` (message + offending token).

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking with `Derive parse failed: …`, and the `Result` twin
  returns null instead of `Err(String)`: Dart lanes propagate
  engine errors (the `toml-lexer` precedent) rather than wrapping
  them.
- The depth tests run in-process instead of on rigged worker
  threads (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean `GrammarParseError` — never a
  `StackOverflowError` — which proves the guard fires first.

## Validation

- **Unit** (`test/derive_parser_test.dart`): all 16 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, the exact 14-level depth boundary across
  0–30, valid-expression parsing, assignment and equation shapes,
  API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `program`, and both APIs always agree.
- **Coverage**: 100% lines and functions.
