# coding_adventures_dartmouth_basic_lexer (Dart)

Dartmouth BASIC lexer driven by the compiled `dartmouth_basic.tokens`
grammar. The Dart lane of the Rust `dartmouth-basic-lexer` crate —
one language definition
(`code/grammars/dartmouth_basic/dartmouth_basic.tokens`) powers
every implementation.

```dart
import 'package:coding_adventures_dartmouth_basic_lexer/dartmouth_basic_lexer.dart';

final tokens = tokenizeDartmouthBasic('10 LET X = 42\n');
print([for (final t in tokens) t.type]);
// [LINE_NUM, KEYWORD, NAME, EQ, NUMBER, NEWLINE, EOF]
```

## API

- `tokenizeDartmouthBasic(source)` tokenizes in one shot, returning
  the tokens with grammar-name types verbatim. Like the reference,
  this lane offers no `Result`/nullable twin: use
  `createDartmouthBasicLexer(source).tokenize()` when the call site
  needs to handle errors explicitly.
- `createDartmouthBasicLexer(source)` returns the underlying
  `GrammarLexer` with the Dartmouth BASIC grammar and both
  post-tokenize hooks registered.
- `relabelLineNumbers(tokens)` promotes a line-leading `NUMBER` to
  `LINE_NUM`; `suppressRemContent(tokens)` drops everything between
  `REM` and its line's end. Both are exported like the reference.
- Lex failures throw the engine's `LexerError` (message, line,
  column, stable error id).

## Deliberate divergences from the reference

- Errors propagate as the structured `LexerError` instead of
  panicking: Dart lanes propagate engine errors (the `toml-lexer`
  precedent) rather than wrapping them.

## Faithful sharp edges (proven identical to the reference)

- A `REM` swallowing to end-of-input swallows the trailing `EOF`
  too (`10 REM foo` yields `[LINE_NUM, KEYWORD]` with no EOF) —
  verified byte-identical against the Rust crate via a scratch
  binary; the property and fuzz EOF oracles encode this exactly.
- The grammar's `errors:` section is dormant in both engines: `@`
  throws rather than emitting `UNKNOWN`.

## Validation

- **Unit** (`test/dartmouth_basic_lexer_test.dart`): all 52
  reference tests, plus the REM-at-EOF edge.
- **Property** (`test/properties_test.dart`): 9 seeded properties over
  the in-lane `pbt` package — closed vocabulary, reference EOF
  shape, determinism, monotonic positions, the relabel rule replayed
  independently over random programs, REM suppression, keyword case
  behavior, operator atomicity, factory agreement.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `LexerError` escapes, both hook
  invariants hold on every run, and the factory path always agrees.
- **Coverage**: 100% lines and functions.
