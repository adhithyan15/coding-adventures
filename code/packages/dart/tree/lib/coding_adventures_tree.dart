/// A rooted tree backed by a directed graph.
///
/// The Dart lane of `tree` (spec DT02): [Tree] enforces one root, one
/// parent per node, and no cycles over the in-lane directed graph, with
/// traversals, queries, lowest common ancestor, subtree extraction, and
/// ASCII visualization. Violations throw [TreeError].
library coding_adventures_tree;

export 'src/tree.dart';
