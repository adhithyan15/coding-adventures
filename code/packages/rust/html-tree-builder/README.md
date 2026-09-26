# html-tree-builder

HTML tree construction written the way the WHATWG HTML specification
describes it: §13.2.4 (parse state) to §13.2.6 (tree construction), with one
named type per specification concept and one method per insertion mode, in
the specification's order. It is Venture's next tree builder (spec
[BR03](../../../specs/BR03-html-tree-builder.md), roadmap phase BR02 P2).

## Where it sits

```text
source ──▶ html-lexer ──tokens──▶ html-tree-builder ──▶ dom_core::Document ──▶ Venture
               ▲                        │
               └── tokenizer switches ──┘   (RCDATA after <title>, …)
```

`html-parser` has a tree builder of its own that passes almost all of the
html5lib corpus, but it is built from flags and post-parse repair passes
rather than the specification's state machine (BR03 §1). This crate grows
beside it, measured on the same corpus, until it is at least as correct; then
`html-parser` delegates to it and the repair passes are deleted.

## What is written

| specification | here |
|---|---|
| §13.2.4.1 insertion mode (21 modes) | `InsertionMode` |
| §13.2.4.2 stack of open elements, scopes | `open_elements::OpenElements` |
| §13.2.4.3 list of active formatting elements, Noah's Ark | `active_formatting::ActiveFormatting` |
| §13.2.6.1 appropriate place for inserting, foster parenting | `TreeBuilder::appropriate_place` |
| §13.2.6.4.1–7 initial … in body, including select | `TreeBuilder::initial` … `in_body` |
| §13.2.6.4.7 adoption agency algorithm | `TreeBuilder::adoption_agency` |
| §13.2.6.4.8 text | `TreeBuilder::text` |
| §13.2.6.4.9–15 in table … in cell, foster parenting | `in_table` … `in_cell` |
| §13.2.6.4.16 in template, §13.2.6.4.17–21 after body … after after frameset | one method each |
| §13.2.6 dispatcher, §13.2.6.5 foreign content (SVG, MathML, integration points, CDATA) | `TreeBuilder::dispatch`, `foreign_content` |

Not yet (BR03 §5): parse errors by specification code, and the switch-over
from `html-parser`.

## Resource limits

Untrusted pages must not be able to exhaust memory or time. Three limits sit
on top of the specification's rules, each reported as a diagnostic when hit
and far beyond anything in the corpus:

- **Output depth 512** (`arena::MAX_TREE_DEPTH`, Blink's figure), applied
  once where the tree leaves the arena: an element at the limit is emitted
  without children, which follow it as siblings. However the tree was built —
  the adoption agency can re-nest elements after the fact — no consumer
  receives a deeper tree. `tree-builder-depth-limit`.
- **512 open elements** (`tree_builder::MAX_OPEN_ELEMENTS`). A start tag that
  finds the stack full first closes the current node, as if its end tag had
  been omitted. Every walk of the stack is then bounded, so parse time stays
  linear in the input. `tree-builder-open-elements-limit`.
- **64 active formatting elements after the last marker**
  (`active_formatting::MAX_ENTRIES_AFTER_MARKER`). Distinct attributes defeat
  the Noah's Ark clause, and every text insertion reconstructs the whole list,
  so without it `<b id=1><b id=2>…` amplifies input quadratically.
  `tree-builder-formatting-limit`.

Membership in the stack and the formatting list is a hash lookup, and
positions are searched from the end, where the builder's targets are.

## Progress

`tests/corpus.rs` runs html-parser's html5lib corpus (2,655 cases). **2,614
pass**, including `template.dat:124`, which `html-parser` fails. Every failing case is listed in `tests/fixtures/expected-failures.txt`;
the test fails on an unlisted failure and on a listed case that passes, so the
list only shrinks. The 41 that fail are all outside this crate. Two groups are outside this crate: 31 script-data cases wait on an
`html-lexer` fix, and 4 `<selectedcontent>` cases wait on a real DOM.

## Usage

```rust
use coding_adventures_html_tree_builder::{html5lib, parse_document, TreeBuilderOptions};

let output = parse_document("<p>One<p>Two", TreeBuilderOptions::default())?;
for line in html5lib::document_lines(&output.document) {
    println!("{line}");
}
```

## Testing

```bash
cargo test -p coding-adventures-html-tree-builder
# After a change that fixes cases, rewrite the list and check it only shrank:
HTML_TREE_BUILDER_BLESS=1 cargo test -p coding-adventures-html-tree-builder --test corpus
# Show expected and actual trees for particular cases:
HTML_TREE_BUILDER_SHOW=tests1.dat:30,webkit02.dat:47 cargo test -p coding-adventures-html-tree-builder --test corpus show_selected -- --nocapture
```
