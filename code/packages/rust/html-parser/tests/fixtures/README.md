# html-parser fixtures

`html5lib-tree-construction-smoke.dat` is the checked-in Venture smoke corpus
mirrored from the currently audited
`html5lib/html5lib-tests/tree-construction/*.dat` sources.

The format is the upstream tree-construction test format documented in
`html5lib/html5lib-tests/tree-construction/README.md`. Keeping the fixture in
that format lets this crate grow toward WHATWG tree-construction compliance
without inventing a Rust-only test schema.

`audit_html5lib_coverage.py` compares this fixture and the sibling lexer
html5lib fixtures against an upstream html5lib-tests checkout by source
signature:

```bash
HTML5LIB_TESTS_ROOT=/path/to/html5lib-tests \
  python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py
```

The command exits nonzero if any upstream tree-construction or tokenizer source
case is missing, or if the normalized tokenizer fixture still has skipped
runtime gaps.
