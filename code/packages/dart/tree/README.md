# coding_adventures_tree (Dart)

A rooted tree backed by a directed graph, with traversals, lowest common
ancestor, subtree extraction, and ASCII visualization. The Dart lane of the
Rust `tree` crate (spec DT02).

```dart
import 'package:coding_adventures_tree/coding_adventures_tree.dart';

final t = Tree('Program');
t.addChild('Program', 'Assignment');
t.addChild('Assignment', 'Name');
print(t.toAscii());
// Program
// └── Assignment
//     └── Name
```

## API

- `Tree(root)` — one root, one parent per node, no cycles; edges point
  parent → child over the in-lane directed graph.
- Mutation: `addChild` (throws [NodeNotFound]/[DuplicateNode]),
  `removeSubtree` (prunes the branch; throws [NodeNotFound]/[RootRemoval]).
- Queries: `root`, `parent`, `children` (sorted), `siblings`, `isLeaf`,
  `isRoot`, `depth`, `height`, `size`, `nodes`, `leaves`, `hasNode`.
- Traversals: `preorder`, `postorder`, `levelOrder` (children sorted).
- Utilities: `pathTo`, `lca`, `subtree` (independent copy), `toAscii`,
  `graph` (detached snapshot copy: inspect freely, mutations cannot reach
  the tree — the reference's shared borrow has no Dart equivalent).
- [TreeError] — sealed `NodeNotFound`/`DuplicateNode`/`RootRemoval` with
  exact reference message texts.

Display strings quote node names with plain double quotes; names needing
escapes render slightly differently than the reference's Rust debug
format (documented; untested upstream either).

## Validation

- **Unit** (`test/tree_test.dart`): mirrors the reference suite
  case-for-case across its eleven categories, plus error value semantics,
  error precedence, result-list independence, name edges, snapshot
  detachment, and 20k-deep traversal survival. 140 tests.
- **Property** (`test/properties_test.dart`): structural invariants,
  traversal laws, path/depth/LCA agreement, subtree consistency,
  remove-restoration, exact mutation errors — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration script-driven
  differential campaign (adds, prunes, queries) against an independent
  parent-map model predicting every observable after every op. Ad-hoc
  60,000-iteration runs over three more seeds: zero divergences.
- **Coverage**: 100% lines, branches, and functions.
