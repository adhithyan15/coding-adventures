# coding_adventures_mccarthy_lisp_parser (Dart)

McCarthy Lisp 1.0 parser driven by the compiled
`mccarthy_lisp.grammar` grammar. The Dart lane of the Rust
`mccarthy-lisp-parser` crate — one language definition
(`code/grammars/mccarthy_lisp`) powers every implementation.

```dart
import 'package:coding_adventures_mccarthy_lisp_parser/mccarthy_lisp_parser.dart';

final forms = parseMcCarthy("(CAR '(A B C))");
print(forms.single);
// (CAR . ((QUOTE . ((A . (B . (C . NIL))) . NIL)) . NIL))
```

## API

- `parseMcCarthy(source)` tokenizes with the Dart mccarthy-lisp-lexer
  lane and parses into the top-level form sequence
  (`List<LispExpr>`).
- `tryParseMcCarthy(source)` is the nullable twin: `null` instead of
  throwing (mirrors the reference's `Result`-returning `parse` with
  idiomatic Dart nullability).
- `parseMcCarthyToCst(source)` is the lower-level entry point,
  returning the generic `ASTNode` CST rooted at `program`.
- `createMcCarthyParserFromTokens(tokens)` builds a `GrammarParser`
  from a pre-tokenized stream for editor/LSP integrations.
- `extractProgram(root)` lowers a `program` CST to the typed forms.
- `mccarthyGrammar()` exposes the compiled `ParserGrammar`.
- All failures throw `McCarthyParseError` (message + 1-based line and
  column), wrapping lexer, grammar, and extractor errors alike.

## DoS hardening

Three independent depth guards, mirroring the reference:

- The nesting pre-scan rejects sources whose combined paren +
  pending-quote depth exceeds `maxParenDepth` (64) before the grammar
  engine runs — a bare quote run has paren-depth 0 but unbounded
  parser recursion, so both shapes are counted.
- The engine-level rule cap trips list nesting past 59 levels and
  quote chains past 88, protecting direct
  `createMcCarthyParserFromTokens` callers that bypass the pre-scan.
- The extractor refuses CSTs nesting past `maxAstDepth` (256).

## Deliberate divergences from the reference

- Errors throw a structured `McCarthyParseError` (named for the
  language because the parser engine already owns `ParseError`)
  instead of returning `Result`, with a nullable twin for the main
  entry point; the engine's nullable error token falls back to 1:1
  (unreachable with this grammar — every engine failure carries the
  offending token).
- Rule names in extractor diagnostics render double-quoted, matching
  the reference's debug format on the simple identifiers this grammar
  produces.
- The reference's iterative `Drop` has no counterpart: Dart's tracing
  collector reclaims deep trees without a recursive destructor, so
  there is nothing to overflow. The 100k-element flat-list test pins
  the build side.
- The depth tests run in-process instead of on rigged worker threads
  (Dart has no thread-stack tuning): 5000-deep input must still fail
  with a clean `GrammarParseError` — never a `StackOverflowError` —
  which proves the guard fires first.

## Display and the depth caps

Display always prints the fully-dotted cons form, which nests deeper
than the source: a flat 60-element list displays 60 paren levels
deep. Displayed output is always grammatically valid and quoteless,
so only the depth caps can reject it — re-parsing a display succeeds
iff it nests at most 59 deep (the engine list cap binds first), and
then displays identically. The property suite pins this exactly.

## Validation

- **Unit** (`test/mccarthy_lisp_parser_test.dart`): 59 tests mirroring
  the reference suite (22 unit, 6 depth-guard, 26 integration, exact
  duplicates folded), plus API and hand-built-CST extractor-shape
  tests.
- **Property** (`test/properties_test.dart`): 11 seeded properties over
  the in-lane `pbt` package — closed vocabulary, twin totality,
  determinism, the display re-parse law, atom and sexpr parsing, both
  exact depth boundaries across wide ranges, API agreement, error
  positions, constructor laws.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`; only `McCarthyParseError` escapes, every
  failure is shaped and API-consistent, every success re-parses
  deterministically with a lawful display.
- **Coverage**: 100% lines and functions.
