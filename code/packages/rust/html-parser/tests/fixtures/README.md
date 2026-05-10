# html-parser fixtures

`html5lib-tree-construction-smoke.dat` is a small checked-in subset of
`html5lib/html5lib-tests/tree-construction/tests1.dat`.

The format is the upstream tree-construction test format documented in
`html5lib/html5lib-tests/tree-construction/README.md`. Keeping the fixture in
that format lets this crate grow toward WHATWG tree-construction compliance
without inventing a Rust-only test schema.
