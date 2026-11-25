## 0.1.0

- Initial Dart lane of the tree identity: `Tree` over the in-lane
  directed-graph package with add/remove, parent/children/siblings/depth/
  height/leaf queries, preorder/postorder/level-order traversals, `pathTo`,
  `lca`, independent `subtree` copies, ASCII visualization, and the
  `NodeNotFound`/`DuplicateNode`/`RootRemoval` error type.
- Tests mirror the reference suite (138 unit tests), add 6 seeded property
  tests over the in-lane `pbt` package and a 2,000-iteration differential
  fuzz campaign over the in-lane `fuzz` package; 100% line/branch/function
  coverage.
