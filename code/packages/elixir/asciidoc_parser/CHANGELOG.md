# Changelog

## 0.1.0 — Initial release

- AsciiDoc parser producing Document AST nodes
- Block-level parsing: headings (1–6), paragraphs, code blocks, literal blocks,
  passthrough blocks, quote blocks, unordered lists (nested), ordered lists,
  thematic breaks, and comment lines (skipped)
- Inline parsing: strong (`*bold*`, `**bold**`), emphasis (`_italic_`, `__italic__`),
  code spans, link macros (`link:url[text]`), image macros (`image:url[alt]`),
  cross-references (`<<anchor,text>>`), bare URL autolinks, hard breaks, and soft breaks
- `[source,lang]` attribute lists propagate language to code blocks
- Recursive quote block parsing (inner content re-parsed as blocks)
- Nested list support via `build_nested_list/2`
