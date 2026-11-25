# coding_adventures_c_parser (Dart)

C parser driven by the compiled `c.grammar` grammar. The Dart lane
of the Rust `c-parser` crate — one language definition
(`code/grammars/c/c.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_c_parser/c_parser.dart';

final ast = parseC('int main(void) { return 2 + 3; }\n');
print(ast.ruleName);
// translation_unit
```

## API

- `parseC(source)` strips legacy directive lines, checks the token
  budget, and parses into an `ASTNode` rooted at `translation_unit`.
- `tryParseC(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning `try_parse_c`
  with idiomatic Dart nullability).
- `createCParser(source)` returns the underlying `GrammarParser`
  with the C grammar and the recursion-depth guard (`maxRuleDepth`
  128) enabled. Like the reference factory, it does NOT
  budget-check: use `parseC`/`tryParseC` for untrusted input.
- `tryParseCTokens(tokens)` parses a pre-tokenized stream (never
  reconstructs source, never strips directives) after enforcing the
  token budget; `null` on any failure.
- `checkTokenBudget(tokens, …)` enforces the input budget
  (non-empty, token count, no premature EOF, per-field and
  aggregate UTF-8 caps), returning the refusal message or `null`.
  `stripLegacyDirectiveLines(tokens)` drops `HASH`-led lines.
- Lex failures throw the engine's `LexerError`; syntax failures and
  budget refusals throw `GrammarParseError`.

## Deliberate divergences from the reference

- Errors propagate as the structured engine errors instead of
  panicking, and the `Result` twins return null instead of
  `Err(String)`: Dart lanes propagate engine errors (the
  `toml-lexer` precedent) rather than wrapping them.
- The budget sums value + type strings per token where the
  reference sums value + `type_name` + `cv`: Dart tokens carry
  neither metadata field, but `type` is a free string here (the
  reference's base type is a fixed enum), so checking it is the
  fail-closed adaptation.
- The depth test runs in-process instead of on a rigged worker
  thread (Dart has no thread-stack tuning): 5000-deep input must
  still fail with a clean error — never a `StackOverflowError` —
  which proves the guard fires first.

## Validation

- **Unit** (`test/c_parser_test.dart`): all 19 reference tests.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, root shape,
  determinism, positions, the exact 21-level depth boundary across
  0–40 (measured identical in both engines), valid-program
  parsing, directive stripping, budget refusals, API agreement
  across all three entry points.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only engine errors escape, every success
  roots at `translation_unit`, and all entry points agree.
- **Coverage**: 100% lines and functions.
