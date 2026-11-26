## 0.1.0

- Initial Dart lane of the asciidoc-parser identity (spec TE03):
  two-phase AsciiDoc to Document AST parsing — the block state
  machine (headings, paragraphs, ordered/unordered nested lists,
  code/literal/passthrough/quote fences, comments, `[source,lang]`
  attributes) over `coding_adventures_document_ast` nodes, plus the
  byte-faithful inline scanner (constrained/unconstrained spans,
  verbatim code spans, link/image macros, cross-references,
  URL/autolink discrimination, hard/soft breaks). Total and lenient:
  unclosed blocks run to EOF, unclosed markup stays literal, `parse`
  never throws. First Dart consumer of the document-ast lane.
- Tests mirror the reference suite (all 34 behavioral tests) and add
  the untested-corner matrix, 8 seeded property tests over the
  in-lane `pbt` package, and a 2,000-iteration hostile fuzz campaign
  over the in-lane `fuzz` package; 100% line coverage.
