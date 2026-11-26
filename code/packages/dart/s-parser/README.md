# coding_adventures_s_parser (Dart)

S parser driven by the compiled `s.grammar` grammar. The Dart lane
of the Rust `s-parser` crate — one language definition
(`code/grammars/s/s.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_s_parser/s_parser.dart';

final ast = parseS('x <- c(1, 2, 3)\nmean(x)\n');
print(ast.ruleName);
// program
```

## API

- `parseS(source)` tokenizes with the Dart s-lexer lane and parses
  into an `ASTNode` rooted at `program`.
- `tryParseS(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning `try_parse_s`
  with idiomatic Dart nullability).
- `createSParser(source)` returns the underlying `GrammarParser`
  with the S grammar and the recursion-depth guard (`maxRuleDepth`
  200) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures
  throw `GrammarParseError` (message + offending token).

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking, and the `Result` twin returns null instead of
  `Err(String)`: Dart lanes propagate engine errors (the
  `toml-lexer` precedent) rather than wrapping them.
- The depth tests run in-process instead of on rigged worker
  threads (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean `GrammarParseError` — never a
  `StackOverflowError` — which proves the guard fires first.

## Validation

- **Unit** (`test/s_parser_test.dart`): all 24 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, the exact 15-level depth boundary across
  0–30, valid-program parsing, assignment spellings, call shapes,
  API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `program`, and both APIs always agree.
- **Coverage**: 100% lines and functions.
