# CommonMark 0.31.2 portable conformance foundation

This directory is the language-neutral source of truth for CommonMark parser
and TE00 HTML-renderer conformance.

- `spec.json` is the exact 140,487-byte official CommonMark 0.31.2 corpus from
  <https://spec.commonmark.org/0.31.2/spec.json>. It contains 652 Markdown/HTML
  examples and has SHA-256
  `d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20`.
- `provenance.json` pins the upstream version, publication date, author,
  CC BY-SA 4.0 license, artifact size, digest, and example count.
- `renderer-cases.json` contains sixteen independently authored TE00
  Document-AST/options/HTML triples covering all nineteen node types.
- The three `*-schema.json` files close the transport shapes. The standard
  library validator enforces the stricter cross-field, digest, Unicode, and
  bounded-work rules.

Run the complete neutral validation from the repository root:

```sh
python3 code/scripts/commonmark_conformance.py
python3 -m unittest discover -s code/scripts/tests -p test_commonmark_conformance.py
```

The validator is deliberately offline and process-free. It never downloads the
official artifact, executes Markdown, renders in a browser, reads fixture-owned
paths, or runs plugins. Raw HTML in renderer vectors is inert JSON text.

The neutral foundation grants no native-language conformance credit. A parser
must dynamically consume all 652 canonical examples through its public API; a
renderer must dynamically consume all sixteen renderer cases. Existing package
fixture copies remain in place until separately reviewed reader migrations.

The unconditional metadata-contract CI job runs the validator and its
falsification suite even when the package planner selects no native package.
Workflow self-validation also admits Venture acceptance to the existing Node
and pinned npm setup; see the recovery receipt for actual generated frontend
validation and its limits.

See [TE01 portable CommonMark conformance](../../TE01-commonmark-portable-conformance.md)
for normative rules and [VALIDATION.md](VALIDATION.md) for the durable local
receipt.
