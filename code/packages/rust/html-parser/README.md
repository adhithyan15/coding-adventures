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
- implied document shell creation for omitted `html`, `head`, and `body`
- explicit `head` to `body` boundary recovery when body starts or body content
  appears before `</head>`
- duplicate open `body` start-tag recovery that merges missing attributes
  without creating nested body elements
- implied table `tbody` and `tr` creation for common omitted table structure
- implied table `colgroup` creation for bare `col` elements
- table caption/column-group boundary recovery before bare columns, rows, and
  sections
- parser-controlled lexer handoff for `title`, `textarea`, RAWTEXT elements,
  `script`, and `plaintext`
- parser options for scripting-sensitive tokenizer handoff, including
  `noscript`
- parser-approved initial tokenizer contexts for data-state documents and
  foreign-content CDATA or script-state fragments
- simple implied end tags for `p`, `li`, `dt`, `dd`, `option`, `optgroup`,
  ruby annotations, heading elements, and paragraph/block boundaries
- parser diagnostics for unmatched end tags

Future batches can layer the full WHATWG HTML tree-construction insertion modes
onto the same DOM target. A separate adapter can project DOM into
`document-ast` for existing native document rendering.

## Usage

```rust
use coding_adventures_html_lexer::HtmlScriptingMode;
use coding_adventures_html_parser::{
    parse_html, parse_html_with_options, HtmlInitialTokenizerContext, HtmlParseOptions,
};
use dom_core::Node;

let document = parse_html("<p>Hello <strong>Venture</strong></p>").unwrap();

match &document.children[0] {
    Node::Element(element) => assert_eq!(element.name, "html"),
    other => panic!("expected element, got {other:?}"),
}

let no_script_document = parse_html_with_options(
    "<noscript><p>Fallback</p></noscript>",
    HtmlParseOptions {
        scripting: HtmlScriptingMode::Disabled,
        ..HtmlParseOptions::default()
    },
)
.unwrap();

let foreign_cdata_fragment = parse_html_with_options(
    "<svg:title>&amp;</svg:title>]]>",
    HtmlParseOptions {
        initial_tokenizer_context: HtmlInitialTokenizerContext::ForeignContentCdataSection,
        ..HtmlParseOptions::default()
    },
)
.unwrap();

let script_fragment = parse_html_with_options(
    "if (a < b) { run(); }</script><p>done</p>",
    HtmlParseOptions {
        initial_tokenizer_context: HtmlInitialTokenizerContext::ScriptData,
        ..HtmlParseOptions::default()
    },
)
.unwrap();
```
