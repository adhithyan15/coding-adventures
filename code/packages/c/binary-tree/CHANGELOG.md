# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `binary-tree` crate (DT03): a generic
  binary tree with traversals and shape predicates.
- Construction: `bt_new`, `bt_node_new` + `bt_with_root` (manual), and
  `bt_from_level_order` (level-order layout with a `present[]` gap array).
- Shape predicates `bt_is_full` / `bt_is_complete` (breadth-first) /
  `bt_is_perfect`; `bt_height`, `bt_size`.
- Traversals `bt_inorder` / `bt_preorder` / `bt_postorder` (depth-first) and
  `bt_level_order` (breadth-first) into a caller buffer; `bt_to_array`
  (level-order with gaps); `bt_to_ascii` (malloc'd indented diagram).
- `bt_find` / `bt_left_child` / `bt_right_child` lookups.
- Overflow-guarded growable string buffer and BFS queue; checked index
  arithmetic in the level-order build and array fill.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) mirroring the Rust
  crate's own vectors, including the exact ASCII diagram.
