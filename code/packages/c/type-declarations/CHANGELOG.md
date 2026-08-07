# Changelog

All notable changes to the C `type-declarations` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `type-declarations` crate.
- `TdKind` value type (nine variants) with `td_kind_to_iir_hint` /
  `is_concrete_hint` / `equals` / `copy` / `free`; `TdField` / `TdVariant` and
  the `TdNamedType` tagged union (record / union / alias).
- `TypeDeclarations` with named-type and global maps, `td_resolve` (alias-chain
  resolution depth-limited to 32, returning `Any` on a cycle), and
  `td_union_variants`.
- `TdAnnotatedNode` / `TdAnnotatedChild` recursive tree with `iir_hint`,
  `child_node`, `node_children`, and `position`.
- Out-parameter / status-code API (`0` / `-1`) in place of the Rust owned
  values and `Option`; every owning value pairs a constructor with a `*_free`,
  and growable arrays guard their reallocation against `size_t` overflow.
- 80 checks mirroring the Rust crate's tests (to_iir_hint, resolve incl. the
  cycle guard, union_variants) plus annotated-tree coverage, run under every
  available C compiler via the shared `iso-harness`; the suite also passes
  under ASan + UBSan.
