# coding_adventures_r_parser (Dart)

R parser driven by the compiled `r.grammar` grammar. The Dart lane
of the Rust `r-parser` crate — one language definition
(`code/grammars/r/r.grammar`) powers every implementation. Rule
names match `s.grammar` so the shared S evaluator can walk R trees
unchanged.

```dart
import 'package:coding_adventures_r_parser/r_parser.dart';

final ast = parseR('data_frame <- c(1, 2, 3)\nmean(data_frame)\n');
print(ast.ruleName);
// program
```

## API

- `parseR(source)` tokenizes with the Dart r-lexer lane and parses
  into an `ASTNode` rooted at `program`.
- `tryParseR(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning `try_parse_r`
  with idiomatic Dart nullability).
- `createRParser(source)` returns the underlying `GrammarParser`
  with the R grammar and the recursion-depth guard (`maxRuleDepth`
  200) enabled.
- Lex failures throw the engine's `LexerError`; syntax failures
  throw `GrammarParseError` (message + offending token).

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking with `R parse failed: …`, and the `Result` twin returns
  null instead of `Err(String)`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.
- The depth tests run in-process instead of on rigged worker
  threads (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean `GrammarParseError` — never a
  `StackOverflowError` — which proves the guard fires first.

## Validation

- **Unit** (`test/r_parser_test.dart`): all 16 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, the exact 14-level depth boundary across
  0–30, valid-program parsing, assignment spellings, typed NA
  atoms, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `program`, and both APIs always agree.
- **Coverage**: 100% lines and functions.
