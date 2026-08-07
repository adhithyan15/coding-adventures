# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `binary-tree` crate
  (DT03), in namespace `ca`: a generic `BinaryTree<T>` with traversals and shape
  predicates.
- Construction: `make_node` + `with_root` (manual) and `from_level_order`
  (`std::vector<std::optional<T>>` layout). Value semantics (deep copy).
- Shape predicates `is_full` / `is_complete` (breadth-first) / `is_perfect`;
  `height`, `size`.
- Traversals `inorder` / `preorder` / `postorder` (depth-first) and
  `level_order` (breadth-first) returning `std::vector<T>`; `to_array`
  (`std::vector<std::optional<T>>` with gaps); `to_ascii` (indented diagram via
  `operator<<`).
- `find` / `left_child` / `right_child` lookups.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) mirroring the Rust
  crate's vectors, including the exact ASCII diagram and a `BinaryTree<std::string>`
  generic case.
