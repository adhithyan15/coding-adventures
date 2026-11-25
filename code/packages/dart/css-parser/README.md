# coding_adventures_css_parser (Dart)

CSS parser driven by the compiled `css.grammar` grammar. The Dart
lane of the Rust `css-parser` crate — one language definition
(`code/grammars/css/css.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_css_parser/css_parser.dart';

final ast = parseCss('body { color: red; }\n');
print(ast.ruleName);
// stylesheet
```

## API

- `parseCss(source)` tokenizes with the Dart css-lexer lane and
  parses into an `ASTNode` rooted at `stylesheet`.
- `createCssParser(source)` returns the underlying `GrammarParser`
  with the CSS grammar and the recursion-depth guard
  (`maxRuleDepth` 170) enabled — the documented error path, for
  callers that handle errors explicitly. Like the reference, this
  lane offers no `Result`/nullable twin.
- Lex failures throw the engine's `LexerError`; syntax failures
  throw `GrammarParseError` (message + offending token).

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking: Dart lanes propagate engine errors (the `toml-lexer`
  precedent) rather than wrapping them.
- The depth tests run in-process instead of on rigged worker
  threads (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean error — never a `StackOverflowError` —
  which proves the guard fires first.

## Validation

- **Unit** (`test/css_parser_test.dart`): all 13 reference tests,
  plus the five measured depth boundaries pinned exactly (33/34,
  39/40, 55/56, 79/80, 81/82 — all verified identical in Dart).
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, three depth boundaries across wide
  ranges, valid-rule parsing, factory agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `stylesheet`, and the factory path always agrees.
- **Coverage**: 100% lines and functions.
