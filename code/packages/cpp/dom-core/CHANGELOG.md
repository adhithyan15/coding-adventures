# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `dom-core` crate, in
  namespace `ca::dom`: a small DOM tree model.
- `Node` as a `std::variant<DocumentType, Element, Text, Comment>` with static
  factories (`element`, `namespaced_element`, `text`, `comment`) and
  `children()` / `children_mut()` accessors (nullptr for non-elements).
- `Document` with `push_child`; `Element::attribute` returning a borrowed
  `const std::string*`; `Attribute`, `DocumentType`, `Text`, `Comment` structs.
  Value semantics throughout.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): building a document with
  every node kind, attribute lookup, nested children, namespaced elements, and
  doctype optional fields — mirroring the Rust crate's test.
