# CommonMark 0.31.2 fixture provenance

`spec.json` is the official CommonMark 0.31.2 example corpus published at
<https://spec.commonmark.org/0.31.2/spec.json>.

- Records: 652, numbered consecutively from 1 through 652
- SHA-256: `d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20`
- Repository source copy:
  `code/packages/typescript/commonmark/tests/fixtures/spec.json`

The fixture is inert JSON. Tests load every record dynamically and execute it
through the public C# `Commonmark.Parse` and `Commonmark.ToHtml` methods.
