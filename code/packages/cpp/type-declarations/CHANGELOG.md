# Changelog

All notable changes to the C++ `type-declarations` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `type-declarations` crate
  (namespace `ca::type_declarations`).
- `KindDecl` (nine variants) with `to_iir_hint` / `is_concrete_hint` /
  `operator==`; `FieldDecl` / `VariantDecl` and a `std::variant`-based
  `NamedTypeDecl` (record / union / alias).
- `TypeDeclarations` with `std::unordered_map` named-type and global maps,
  `resolve` (alias-chain resolution depth-limited to 32, `Any` on a cycle), and
  `union_variants` returning `std::optional<std::vector<std::string>>`.
- `AnnotatedNode` / `AnnotatedChild` recursive tree (`iir_hint`, `child_node`,
  `node_children`, `position`); child sub-trees are held by `std::shared_ptr`.
- 38 checks mirroring the Rust crate's tests plus annotated-tree coverage, run
  under every available C++ compiler via the shared `iso-harness`; the suite
  also passes under ASan + UBSan.
