# coding-adventures-html-parser

Incremental Rust HTML parser for Venture.

The parser consumes tokens from `coding-adventures-html-lexer` and builds a
DOM tree from `dom-core`. DOM is the primary browser-facing output because it
preserves element names, attributes, comments, doctypes, and text exactly enough
for later CSS, layout, scripting, and Paint VM work.

This first slice intentionally starts small:

- text, comments, doctypes, start tags, end tags, attributes
- a stack of open elements
- void element handling
- adjacent text merging
- parser-controlled lexer handoff for `title`, `textarea`, RAWTEXT elements,
  `script`, and `plaintext`
- simple implied end tags for `p`, `li`, `dt`, and `dd`
- parser diagnostics for unmatched end tags

Future batches can layer the full WHATWG HTML tree-construction insertion modes
onto the same DOM target. A separate adapter can project DOM into
`document-ast` for existing native document rendering.

## Usage

```rust
use coding_adventures_html_parser::parse_html;
use dom_core::Node;

let document = parse_html("<p>Hello <strong>Venture</strong></p>").unwrap();

match &document.children[0] {
    Node::Element(element) => assert_eq!(element.name, "p"),
    other => panic!("expected element, got {other:?}"),
}
```
