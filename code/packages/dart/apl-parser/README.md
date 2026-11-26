# coding_adventures_apl_parser (Dart)

APL parser driven by the compiled `apl.grammar` grammar. The Dart
lane of the Rust `apl-parser` crate — one language definition
(`code/grammars/apl/apl.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_apl_parser/apl_parser.dart';

final ast = parseApl('A←B←3\n');
print(ast.ruleName);
// program
```

## API

- `parseApl(source)` tokenizes with the Dart apl-lexer lane and
  parses into an `ASTNode` rooted at `program`.
- `tryParseApl(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning `try_parse_apl`
  with idiomatic Dart nullability).
- `createAplParser(source)` returns the underlying `GrammarParser`
  with the APL grammar and the recursion-depth guard
  (`maxRuleDepth` 100) enabled.
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

- **Unit** (`test/apl_parser_test.dart`): all 22 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, both exact depth boundaries across wide
  ranges (47 paren levels, 94 chain terms), valid-program parsing,
  line counts, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `program`, and both APIs always agree.
- **Coverage**: 100% lines and functions.
