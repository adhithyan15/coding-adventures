# coding_adventures_javascript_tokens (Dart)

Backend-agnostic JavaScript pipeline types. The Dart lane of the
Rust `javascript-tokens` crate — the shared vocabulary the JS
lexer, parser, AST, and tooling agree on, with zero runtime
dependencies so it sits at the bottom of the graph.

```dart
import 'package:coding_adventures_javascript_tokens/javascript_tokens.dart';

final v = EsVersion.parse('es2020');
print('${v.asStr()} ${v.compareTo(EsVersion.latest) < 0}');
print('trivia: ${TokenKind.newline.isTrivia}, len: ${const Span(2, 5).length}');
// es2020 true
// trivia: true, len: 3
```

## API

- `EsVersion`: the 14 editions with grammars (`es1`–`es2025`),
  `latest`, `all`, `asStr()`, strict `parse`/`tryParse`
  (empty and unknown strings rejected), chronological
  `compareTo`, and the `UnknownEsVersion` error with the
  reference's exact message.
- `Span(start, end)`: half-open byte ranges with `length`,
  `isEmpty`, lexicographic `compareTo`, and value equality.
- `TokenKind`: sealed hierarchy of the 19 unit kinds plus
  payload-carrying `TokenKindOther(name)`, with `isTrivia`
  (comment/whitespace/newline), `isEof`, and value equality.

## Deliberate divergences from the reference

- `default_is_latest` has no counterpart: Dart has no `Default`
  trait; the lane default is `EsVersion.latest`.
- `Span` stores 64-bit `int` (the whole `u32` domain fits); no
  construction-time validation, exactly like the reference. A
  violated `start <= end` yields a negative `length` instead of
  the reference's debug underflow panic.
- `Span` getters are not const-evaluable (Dart has no `const
  fn`), so const-ness is pinned via canonicalization rather
  than const-evaluated arithmetic.
- `tryParse` is a lane-idiomatic nullable twin the reference
  lacks (`from_str` maps to throwing `parse`).

## Validation

- **Unit** (`test/javascript_tokens_test.dart`): 18 of 19
  reference tests plus 3 lane extras.
- **Property** (`test/properties_test.dart`): 9 seeded properties
  over the in-lane `pbt` package — version round-trips,
  declaration-order agreement, name distinctness, error-message
  shape, span arithmetic, lexicographic/total ordering,
  hash agreement, the trivia table, `Other` payload laws.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; `tryParse` never throws, twins agree,
  span/`Other` laws hold on derived values.
- **Coverage**: 100% lines; all functions except the const enum
  constructor (a coverage-tool artifact — const enum constructors
  emit no hit record; same residue as the `clock`/`markov` lanes).
