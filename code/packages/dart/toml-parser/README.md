# coding_adventures_toml_parser (Dart)

TOML parser driven by the compiled `toml.grammar` grammar. The Dart
lane of the Rust `toml-parser` crate — one language definition
(`code/grammars/toml/toml.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_toml_parser/toml_parser.dart';

final ast = parseToml('[server]\nport = 8080\n');
print(ast.ruleName);
// document
```

## API

- `parseToml(source)` tokenizes with the Dart toml-lexer lane and
  parses into an `ASTNode` rooted at `document`.
- `tryParseToml(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning
  `try_parse_toml` with idiomatic Dart nullability).
- `createTomlParser(source)` / `tryCreateTomlParser(source)` return
  the underlying `GrammarParser` with the TOML grammar and the
  recursion-depth guard (`maxRuleDepth` 165) enabled.
- Failures surface as the payload-blind `TomlParseError`
  (`TomlParseError.lexical` / `TomlParseError.syntax`, same
  messages as the reference enum, never echoing the source).

## Deliberate divergences from the reference

- Errors throw as `TomlParseError` constants (identity-compared)
  instead of panicking, and the `Result` twins return null instead
  of `Err(TomlParseError)`: Dart lanes propagate structured errors
  rather than panicking.
- The depth tests run in-process instead of on rigged worker
  threads (Dart has no thread-stack tuning): 5000-deep input in
  both shapes must still fail with a clean error — never a
  `StackOverflowError` — which proves the guard fires first.

## Validation

- **Unit** (`test/toml_parser_test.dart`): all 26 reference tests.
- **Property** (`test/properties_test.dart`): 8 seeded properties
  over the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, both exact depth boundaries (53 levels
  parse, 54 trip, for nested arrays and nested inline tables alike),
  valid-document parsing, API agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `TomlParseError` escapes, every success
  roots at `document`, and both APIs always agree.
- **Coverage**: 100% lines and functions.
