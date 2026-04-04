# Changelog

All notable changes to `@coding-adventures/asciidoc-parser` will be documented here.

## 0.1.0 — Initial release

- Block parser state machine covering all major AsciiDoc block types:
  headings (= through ======), paragraphs, code/listing blocks (----),
  literal blocks (....), passthrough blocks (++++), quote blocks (____),
  unordered and ordered lists with two-level nesting, thematic breaks ('''),
  and single-line comments (//).
- Inline parser with left-to-right character scanner covering: hard and soft
  line breaks, backtick code spans, strong (** and *), emphasis (__ and _),
  link: and image: inline macros, cross-references (<<...>>), bare URL
  autolinks, and https:// / http:// URLs with optional [text].
- [source,lang] attribute line support for code block language hints.
- Recursive parsing of quote block (____) content.
- Produces a DocumentNode conforming to the @coding-adventures/document-ast spec.
- 30+ unit tests covering all block and inline forms.
