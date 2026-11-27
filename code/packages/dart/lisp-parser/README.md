# coding_adventures_lisp_parser (Dart)

Lisp parser driven by the compiled `lisp.grammar` grammar. The Dart
lane of the Rust `lisp-parser` crate — one language definition
(`code/grammars/lisp/lisp.grammar`) powers every implementation.

```dart
import 'package:coding_adventures_lisp_parser/lisp_parser.dart';

final program = parseLisp('(+ 1 2)');
print(program.single.findAtoms());
// [+, 1, 2]
```

## API

- `parseLisp(source)` tokenizes and parses into a `List<SExpr>`
  forest (one tree per top-level form).
- `parseLispTokens(tokens)` parses a pre-tokenized stream, skipping
  re-tokenization.
- `tryParseLisp(source)` / `tryParseLispTokens(tokens)` are the
  nullable twins: `null` instead of throwing, with idiomatic Dart
  nullability.
- `SExpr` is the sealed typed AST (`SAtom`/`SList`/`SDottedPair`/
  `SQuoted`) with `findAtoms`/`countLists`/`countQuoted` helpers;
  `AtomKind` classifies atoms; `LispParseError` carries the
  message-only failure.
- `maxNestingDepth` (48) is the pre-scan guard: the engine runs
  uncapped (matching the reference), so over-deep input is refused
  up front with a catchable error instead of overflowing the stack.
- `lispParserGrammar()` exposes the compiled rules for tooling.
- All failures throw `LispParseError` (lex errors arrive as
  `Lexer error: …`, mirroring the reference's `From` mapping).

## Deliberate divergences from the reference

- Errors propagate as the thrown structured `LispParseError`
  instead of a `Result`, and the nullable twins return null: Dart
  lanes throw or return null (the lexer/parser precedent) rather
  than modelling `Result`.
- The extractor's `Internal error` messages render the offending
  shape Dart-idiomatically rather than in Rust `{:?}` debug format;
  all 48/49 boundary behavior and the pre-scan message are exact.
- The extractor lives public-in-src (`src/extractor.dart`,
  barrel-hidden) so its fail-closed internal arms pin with
  hand-built CSTs; the reference keeps them private.
