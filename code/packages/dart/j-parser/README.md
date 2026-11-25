# coding_adventures_j_parser (Dart)

J parser driven by the compiled `j.grammar` grammar. The Dart lane
of the Rust `j-parser` crate — one language definition
(`code/grammars/j/j.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_j_parser/j_parser.dart';

final ast = parseJ('(+ - *)A\n');
print(ast.ruleName);
// program
```

## API

- `parseJ(source)` tokenizes with the Dart j-lexer lane and parses
  into an `ASTNode` rooted at `program`.
- `tryParseJ(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning `try_parse_j`
  with idiomatic Dart nullability).
- `createJParser(source)` returns the underlying `GrammarParser`
  with the J grammar and the recursion-depth guard (`maxRuleDepth`
  70) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures
  throw `GrammarParseError` (message + offending token).

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking, and the `Result` twin returns null instead of
  `Err(String)`: Dart lanes propagate engine errors (the
  `toml-lexer` precedent) rather than wrapping them.
- The depth tests run in-process instead of on rigged worker
  threads (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean error — never a `StackOverflowError` —
  which proves the guard fires first.

## Validation

- **Unit** (`test/j_parser_test.dart`): all 33 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, all three exact depth boundaries across
  wide ranges (32 paren levels, 63 chain terms, 61 train teeth),
  valid-program parsing, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `program`, and both APIs always agree.
- **Coverage**: 100% lines and functions.
