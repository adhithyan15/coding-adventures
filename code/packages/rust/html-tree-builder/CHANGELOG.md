# Changelog

All notable changes to the `coding-adventures-html-tree-builder` crate will be
documented in this file.

## Unreleased

- **BR03 step 2: tables.** The seven table insertion modes — in table, in
  table text, in caption, in column group, in table body, in row, in cell —
  with foster parenting and the "clear the stack back to a context" steps.
  Every insertion mode is now written; the `tree-builder-mode-not-implemented`
  fallback is gone. 2,138 of 2,655 corpus cases pass (255 more), and every
  remaining failure needs foreign content, fragment parsing, the html-lexer
  script-data fix or a real DOM. Robustness tests cover foster-parenting
  floods and deeply nested tables.

- **New crate: BR03 step 1.** Tree construction as the WHATWG specification
  writes it, beside `html-parser`:
  - an arena tree with parent links that converts to `dom_core::Document`
    without recursion;
  - `OpenElements` with the default, list-item, button and table scopes;
  - `ActiveFormatting` with markers, the Noah's Ark clause and reconstruction;
  - the dispatcher and every insertion mode outside tables: initial, before
    html, before head, in head, in head noscript, after head, in body
    (formatting elements, the adoption agency algorithm, and the current
    customizable-select rules), text, in template, after body, in frameset,
    after frameset, after after body and after after frameset;
  - quirks-mode detection from the DOCTYPE, and the SVG, MathML and foreign
    attribute adjustment tables;
  - a bound on reprocessing, and a cycle check before the adoption agency
    inserts, so no input can hang the builder;
  - resource limits from two rounds of security review: output depth 512,
    applied where the tree leaves the arena so no construction path can
    exceed it; 512 open elements, a full stack closing its current node
    before the next start tag; 64 active formatting elements after the last
    marker; hash-set membership and search-from-the-end in the stack and the
    formatting list. Hostile inputs that took 2.8 to 452 s (or crashed on
    drop) now parse 250-900 KB in under a second.
- The html5lib corpus harness with a shrink-only expected-failure list:
  1,883 of 2,655 cases pass.
