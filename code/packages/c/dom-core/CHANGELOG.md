# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `dom-core` crate: a small DOM tree model
  (document, element, text, comment, and doctype nodes).
- Constructors `dom_element` / `dom_namespaced_element` / `dom_text` /
  `dom_comment` / `dom_doctype` (malloc'd handles; attribute arrays deep-copied),
  `dom_node_free` (recursive), and `dom_document_new` / `dom_document_free`.
- Ownership-transferring `dom_element_append_child` / `dom_document_push_child`
  (child freed on OOM), and accessors: `dom_node_kind`, element name/namespace/
  attribute/children, text/comment data, and the doctype fields.
- Tagged-union node with recursive free; child arrays guard their growth against
  `size_t` overflow; `calloc` used for the checked attribute-array multiply.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): building a document with
  every node kind, attribute lookup, nested children, namespaced elements, and
  doctype optional fields — mirroring the Rust crate's test.
