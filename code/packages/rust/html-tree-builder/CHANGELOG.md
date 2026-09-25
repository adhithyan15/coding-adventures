# Changelog

All notable changes to the `coding-adventures-html-tree-builder` crate will be
documented in this file.

## Unreleased

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
  - resource limits from the security review: tree depth 512 (a 100,000-deep
    document took 12.7 s and overflowed the stack when dropped; now 0.9 s),
    at most 64 active formatting elements after the last marker (distinct
    attributes defeated the Noah's Ark clause: 75 KB of input took 42 s and
    gigabytes; now 0.2 s), reconstruction by index instead of search, and a
    per-name count of open elements that answers most scope checks at once.
- The html5lib corpus harness with a shrink-only expected-failure list:
  1,883 of 2,654 cases pass.
