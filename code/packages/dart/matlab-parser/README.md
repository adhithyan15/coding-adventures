# coding_adventures_matlab_parser (Dart)

MATLAB parser driven by the compiled `matlab.grammar` grammar. The
Dart lane of the Rust `matlab-parser` crate — one language definition
(`code/grammars/matlab/matlab.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_matlab_parser/matlab_parser.dart';

final ast = parseMatlab('M = [1 2; 3 4]\n');
print(ast.ruleName);
// program
```

## API

- `parseMatlab(source)` tokenizes with the Dart matlab-lexer lane
  and parses into an `ASTNode` rooted at `program`.
- `tryParseMatlab(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_parse_matlab` with idiomatic Dart nullability).
- `createMatlabParser(source)` returns the underlying
  `GrammarParser` with the MATLAB grammar, the `end`-retag pre-parse
  hook, and the recursion-depth guard (`maxRuleDepth` 200) enabled.
- `retagIndexEnd(tokens)` rewrites bracketed `end` keywords to
  `NAME` so index sentinels never collide with block terminators.
- Lex failures throw the engine's `LexerError`; syntax failures
  throw `GrammarParseError` (message + offending token).

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking with `MATLAB parse failed: …`, and the `Result` twin
  returns null instead of `Err(String)`: Dart lanes propagate
  engine errors (the `toml-lexer` precedent) rather than wrapping
  them.
- The depth tests run in-process instead of on rigged worker
  threads (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean `GrammarParseError` — never a
  `StackOverflowError` — which proves the guard fires first.

## Validation

- **Unit** (`test/matlab_parser_test.dart`): all 19 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, the exact 12-level depth boundary across
  0–30, valid-program parsing, end retagging, matrix shapes, API
  agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `program`, and both APIs always agree.
- **Coverage**: 100% lines and functions.
