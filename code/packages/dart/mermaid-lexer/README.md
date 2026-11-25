# coding_adventures_mermaid_lexer (Dart)

Mermaid lexers for all 34 diagram families, driven by the compiled
`mermaid/*.tokens` grammars. The Dart lane of the Rust
`mermaid-lexer` crate — one language definition per family
(`code/grammars/mermaid/`) powers every implementation.

```dart
import 'package:coding_adventures_mermaid_lexer/mermaid_lexer.dart';

final tokens = tokenizeMermaid('flowchart LR\nA[Start] --> B\n');
print(tokens.map((t) => '${t.type}:${t.value}').join(' '));
// KEYWORD:flowchart DIRECTION:LR NEWLINE:\n NAME:A RECT:[Start] ARROW:--> NAME:B NEWLINE:\n EOF:
```

## API

- 34 factories (`createMermaidLexer`, `createMermaidPieLexer`,
  …, `createMermaidGanttLexer`) returning the underlying
  `GrammarLexer` — one per diagram family.
- 9 throwing one-shots (`tokenizeMermaid`, `…Pie`, `…Sankey`,
  `…Gitgraph`, `…Er`, `…C4`, `…Sequence`, `…State`, `…Quadrant`),
  including the sequence keyword-normalization and state
  line-filter post-processors.
- 27 nullable twins (`tryTokenizeMermaidBlock`, …), returning
  `null` instead of throwing (mirroring the reference's
  `Result`-returning `try_tokenize` with idiomatic Dart
  nullability).
- `mermaidVersion` (`'0.84.0'`), mirroring the reference const.

## Deliberate divergences from the reference

- The reference parses each `.tokens` file at runtime on every
  factory call; this lane compiles all 34 grammars at authoring
  time, so construction is just a lookup.
- Failures throw the structured engine `LexerError` instead of
  panicking with a family-prefixed message or returning
  `Err(String)`.

## Validation

- **Unit** (`test/mermaid_lexer_test.dart`): all 135 reference
  tests plus 2 lane extras (the 10 reference-untested families
  lex their headers; all 34 factories construct working lexers).
- **Property** (`test/properties_test.dart`): 8 seeded properties
  over the in-lane `pbt` package — closed vocabulary, EOF shape,
  determinism, positions, per-family valid-snippet lexing,
  factory agreement, twin agreement (all across all 34 families),
  registry distinctness.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`, every input run under all 34 families; only
  `LexerError` escapes, every success ends with `EOF` and re-lexes
  deterministically.
- **Coverage**: 100% lines and functions.
