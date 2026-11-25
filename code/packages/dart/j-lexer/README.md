# coding_adventures_j_lexer (Dart)

J lexer driven by the compiled `j.tokens` grammar. The Dart lane of
the Rust `j-lexer` crate — one language definition
(`code/grammars/j/j.tokens`) powers every implementation.

```dart
import 'package:coding_adventures_j_lexer/j_lexer.dart';

final tokens = tokenizeJ('A=.i.5');
print([(for (final t in tokens) t.type)]);
// [NAME, ASSIGN_LOCAL, IDOT, NUMBER, EOF]
```

## API

- `tokenizeJ(source)` tokenizes in one shot, returning the tokens
  with grammar-name types verbatim plus a final `EOF`.
- `createJLexer(source)` returns the underlying `GrammarLexer` with
  the J grammar.
- Lex failures throw the engine's `LexerError` (message, line, column,
  stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of panicking
  with `J tokenization failed: …`: Dart lanes propagate engine errors
  (the `toml-lexer` precedent) rather than wrapping them.

## Validation

- **Unit** (`test/j_lexer_test.dart`): all 15 reference tests plus
  adverb values, dollar/hash singles, error positions, and empty
  input.
- **Differential** (throwaway probes, removed after): all 39 unit-test
  inputs run through both lanes — streams byte-identical across Rust
  and Dart (machine-checked `diff`, exit 0).
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, monotonic positions, digraph atomicity, number/name
  wholeness, comment invisibility, assignment shape.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes.
- **Coverage**: 100% lines and functions.
