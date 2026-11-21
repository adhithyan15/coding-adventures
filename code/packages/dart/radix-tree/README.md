# coding_adventures_radix_tree

A pure Dart DT14 radix tree (compressed Patricia trie) mapping string keys to generic
values with Unicode-scalar compressed edges, iterative stack-safe traversal, edge
merge-back re-compression on deletion, and payload-blind diagnostic safety.

## Features

- **DT14 Radix Tree Specification**: Edge compression collapses chains of single-child
  nodes into multi-scalar edge labels.
- **Unicode-Scalar Edges**: Edges operate directly on Unicode scalar code points
  (`0x0000..0x10FFFF`), preventing surrogate pair tearing on astral emoji (e.g. `\u{1F600}`).
- **Iterative Traversal**: Insertion, deletion, search, prefix collection, and structural
  invariants use explicit stack frames instead of recursive call stacks, remaining safe
  for keys up to 50,000+ scalars.
- **Deletion Merge-Back**: Deleting keys re-compresses collapsible single-child nodes into
  single edges, maintaining structural invariants.
- **Nullable Value Support**: Explicit `isEnd` flags distinguish absent keys from keys
  storing `null` (`RadixTree<V?>`).
- **Standing Rule #20**: Zero inline string literals in operational logic, closed
  exception hierarchies, and payload-blind error diagnostics.

## 5-Class Radix-Tree Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Deep-Key Stack Overflow & Key-Length DoS** | Call stack exhaustion or runaway allocation from huge keys | `maxKeyScalars = 65536` ceiling enforced atomically before mutation. Iterative algorithms use explicit loops/stacks, safely processing 50,000-scalar keys without call stack recursion. |
| **2: Surrogate-Pair Tearing & Malformed UTF-16 Smuggling** | Corrupted multi-byte characters and torn surrogate pairs | All input strings undergo atomic pre-validation rejecting lone/reversed UTF-16 surrogates (`0xD800..0xDFFF`). Edges are indexed by 32-bit Unicode scalar values, keeping astral emoji (`\u{1F600}` vs `\u{1F601}`) cleanly separated without splitting surrogates. |
| **3: Compression & Re-Compression Structural Integrity** | Dangling single-child chains or uncompressed nodes degrading performance | Handles all 4 DT14 insertion cases (no common prefix, edge prefix, key prefix, partial split) and deletes with merge-back re-compression. Validated continuously via `isValid()`. |
| **4: Nullable Value Ambiguity & Caller-Mutation Isolation** | Ambiguity between absent keys and `null` values; internal corruption from external mutations | Explicit `isEnd` boolean decouples key presence from value nullability. All returned collections (`keys`, `entries`, `wordsWithPrefix`, `keysWithPrefix`, `toMap`) return unmodifiable views/lists. |
| **5: CWE-209 Payload-Blind Confidentiality** | Sensitive keys or values leaked through exceptions or diagnostic strings | Closed exception hierarchy (`InvalidRadixKeyException`, `RadixKeyNotFoundException`) and structural `toString()` emit only static text and redacted placeholders (`<redacted>`), never reflecting caller payload data. |

## Quick Start

```dart
import 'package:coding_adventures_radix_tree/radix_tree.dart';

void main() {
  final tree = RadixTree<int>()
    ..insert('search', 1)
    ..insert('searcher', 2)
    ..insert('searching', 3);

  print(tree.search('search')); // 1
  print(tree.nodeCount());      // 4 (vs 14 in an uncompressed trie)
  print(tree.edgeCount());      // 3

  final match = tree.longestPrefixMatch('searching_for_answers');
  print(match?.key);            // 'searching'
  print(match?.value);          // 3

  tree.delete('searching');
  tree.delete('search');
  print(tree.nodeCount());      // 2 (merged back to 'searcher')
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, test, and coverage (>= 95%)
```
