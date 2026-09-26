# Changelog

All notable changes to the `coding-adventures-html-tree-builder` crate will be
documented in this file.

## Unreleased

- `tests/raw_text_end_tags.rs` pins that hidden-`<img>` payloads around
  raw-text end tags (`</</style>`, quoted `>` in end-tag attributes) produce
  no element.
- **Script-data end tags.** With `html-lexer` deciding appropriate end tags
  at whitespace or `/`, 31 listed corpus cases (`tests16.dat`,
  `domjs-unsafe.dat:145/146`, `scriptdata01.dat:411`)
  pass and leave `tests/fixtures/expected-failures.txt`.

- **BR03 step 4: fragment parsing (§13.4).** `parse_fragment(source,
  &FragmentContext, options)` parses as the children of a context element,
  HTML or foreign (`FragmentContext::html("td")`): the tokenizer starts in the
  context's text state with no appropriate end tag, a `template` context
  pushes "in template", the insertion mode is reset through the context, and
  `</html>` and `</frameset>` take their fragment-case rules. The form rules
  now follow the specification's "parsing template contents" (a template open
  *or* a template context). **2,614 of 2,655** corpus cases pass (199 more),
  including `template.dat:124`, which `html-parser` fails. The 41 that fail:
  31 wait on the html-lexer script-data fix, 6 need a script engine, 4 are
  `<selectedcontent>`.

- **BR03 step 3: foreign content.** The tree construction dispatcher now
  sends tokens to the foreign-content rules (§13.2.6.5) when the adjusted
  current node is SVG or MathML, except at MathML text integration points,
  HTML integration points (`foreignObject`, `desc`, `title`, HTML
  `annotation-xml`) and `<svg>` in `annotation-xml`. The rules: HTML tags that
  break out of foreign content, SVG tag-name and attribute case, MathML and
  foreign (`xlink:`, `xml:`, `xmlns`) attributes, self-closing foreign
  elements, and the end-tag walk. CDATA sections are read back from the
  bogus comment `html-lexer` makes of them, switching the tokenizer to the
  CDATA section state when a section runs past its first `>`.
  **2,415 of 2,655** corpus cases pass (277 more). Every remaining failure is
  a fragment case (step 4), the html-lexer script-data bug, a scripted case,
  or `<selectedcontent>`.
- **Security review of step 3.** Only a real `<![CDATA[` is read back as a
  CDATA section: the driver marks the one comment the lexer made of it
  (counting its `cdata-in-html-content` diagnostics, each scanned once).
  Before, a genuine comment `<!--[CDATA[-->` or a bogus end tag `</[CDATA[x>`
  switched the tokenizer too, so markup every spec parser treats as inert came
  out as a live `<img onerror>`. And the foreign end-tag walk compares names
  with `eq_ignore_ascii_case` instead of allocating a lowercase copy per step
  (1 MB of long names: 14 s → 1 s). Both are pinned in `tests/foreign_content.rs`.

- **BR03 step 2: tables.** The seven table insertion modes — in table, in
  table text, in caption, in column group, in table body, in row, in cell —
  with foster parenting and the "clear the stack back to a context" steps.
  Every insertion mode is now written; the `tree-builder-mode-not-implemented`
  fallback is gone. 2,138 of 2,655 corpus cases pass (255 more), and every
  remaining failure needs foreign content, fragment parsing, the html-lexer
  script-data fix or a real DOM. Robustness tests cover foster-parenting
  floods and deeply nested tables.
- **Children are a doubly linked list** (first/last child, previous/next
  sibling), so inserting before a node, finding the node before one, and
  detaching are O(1). The security review of step 2 found foster parenting
  quadratic with a child vector: every stray node goes in front of the open
  table, and each insert scanned for it (1 MB of `<table>` + `a<br>`... took
  38 s; now 0.85 s).

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
