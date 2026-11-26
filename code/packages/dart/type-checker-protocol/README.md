# coding_adventures_type_checker_protocol (Dart)

Shared type-checker contract and generic checker framework. The Dart lane of
the TypeScript
[`type-checker-protocol`](../../typescript/type-checker-protocol/README.md)
reference package: compiler toolchains get a common semantic-analysis shape.

```dart
import 'package:coding_adventures_type_checker_protocol/coding_adventures_type_checker_protocol.dart';

class ToyChecker extends GenericTypeChecker<ToyNode> {
  ToyChecker() {
    registerHook('node', 'literal', (node, args) {
      node.text = node.text.trim();
      return null;
    });
  }

  @override
  void run(ToyNode ast) => dispatch('node', ast);

  @override
  String? nodeKind(ToyNode node) => node.kind;
}

final result = ToyChecker().check(ast); // typedAst, errors, ok
```

## API

- `TypeChecker<ASTIn, ASTOut>` — the contract: `check(ast)` returns a
  `TypeCheckResult<ASTOut>`.
- `TypeCheckResult<AST>` — `typedAst`, `errors`, `ok` (true iff no errors).
- `TypeErrorDiagnostic` — `message`, `line`, `column`; a value type with
  structural equality.
- `GenericTypeChecker<AST>` — reusable base: `registerHook(phase, kind,
  hook)`, `dispatch(phase, node, [args])` (kind tier first, then the `*`
  tier, first non-declined result wins), `error(message, subject)` with
  `locate(subject)` positions (default (1, 1)), and kind normalization
  (alphanumerics kept, other runs collapse to one `_`, trimmed).
  Subclasses implement `run` and `nodeKind`; all seven extension points are
  `@protected`.

Two adaptations from the reference:

1. **Hook arguments are an explicit list.** TypeScript's variadic
   `...args` has no Dart equivalent, so hooks take `(node, args)` and
   `dispatch` takes an optional positional argument list.
2. **Wildcard registration is fixed.** The reference normalizes `'*'` to
   `""` at registration (storing under `"phase:"`) while `dispatch` looks
   up the literal `"phase:*"` tier, so wildcard hooks never fire there —
   uncovered by the reference suite. Here the exact kind `'*'` is stored
   under the literal `"phase:*"` key, making the tier work as `dispatch`
   intends. (A matching fix for the TypeScript package is logged as
   follow-up work; this PR does not touch it.)

## Validation

- **Unit** (`test/type_checker_protocol_test.dart`): mirrors the reference
  suite, plus wildcard routing, decline chaining, hook order, argument
  forwarding, lifecycle freshness, normalization spellings (kebab, snake,
  padded, punctuation, non-ASCII, astral, blank), and the diagnostic value
  type. 27 tests.
- **Property** (`test/properties_test.dart`): self-routing, first-match-wins
  against an order model, wildcard fallback, args forwarding, cross-check
  freshness, result shape — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential campaign
  against an independent normalization+dispatch model (code units + regex
  trim vs the library's runes + index loops). Ad-hoc 60,000-iteration runs
  over three more seeds: zero divergences.
- **Coverage**: 100% lines, branches, and functions.
