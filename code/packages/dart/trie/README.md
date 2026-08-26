# coding_adventures_trie

A pure Dart implementation of the [DT13 trie contract](../../../specs/DT13-trie.md).
A trie stores string keys one Unicode scalar per edge, making exact lookup,
autocomplete-style prefix queries, and longest-prefix matching natural.

## Usage

```dart
import 'package:coding_adventures_trie/trie.dart';

final routes = Trie<String>()
  ..insert('api', 'API root')
  ..insert('api/v1', 'version one')
  ..insert('assets', 'static files');

assert(routes.search('api') == 'API root');
assert(routes.containsKey('ap') == false);
assert(routes.startsWith('ap') == true);
assert(routes.longestPrefixMatch('api/v1/users') == ('api/v1', 'version one'));

final completions = routes.wordsWithPrefix('a');
// [('api', 'API root'), ('api/v1', 'version one'), ('assets', 'static files')]
```

`insert` updates an existing endpoint without increasing `count`. `delete`
returns whether a key existed and prunes nodes that no longer serve another
key. Empty strings are valid keys. A present key may store `null`; use
`containsKey` when absence must be distinguished from a nullable value.

## Portable key semantics

- Keys are exact Unicode-scalar sequences, traversed with Dart `String.runes`.
- No Unicode normalization or locale collation is performed.
- Ordered results compare scalar values numerically and emit a prefix before
  its descendants.
- `startsWith('')` is true exactly when at least one key is stored.
- Insert, lookup, deletion, traversal, and validation use explicit loops or
  frame stacks, so long keys do not depend on recursive call-stack depth.

The package is self-contained. DT02 provides the conceptual tree model, but no
production tree dependency is imported. The separately tracked Dart LZ78
migration will reconcile its byte-at-a-time cursor with this package rather
than widening this foundational port.

## Authority and development

Production code is deterministic pure computation with no filesystem, network,
process, environment, clock, entropy, console, FFI, or native authority. Run:

```text
dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=90
```
