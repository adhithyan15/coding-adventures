# coding-adventures-html-parser

Incremental Rust HTML parser for Venture.

The parser consumes tokens from `coding-adventures-html-lexer` and builds a
DOM tree from `dom-core`. DOM is the primary browser-facing output because it
preserves element names, attributes, comments, doctypes, and text exactly enough
for later CSS, layout, scripting, and Paint VM work.

The current parser surface includes:

- text, comments, doctypes, start tags, end tags, attributes
- a stack of open elements
- void element handling
- adjacent text merging
- implied document shell creation for omitted `html`, `head`, and `body`
- explicit `head` to `body` boundary recovery when body starts or body content
  appears before `</head>`
- duplicate open `body` start-tag recovery that merges missing attributes
  without creating nested body elements
- duplicate open `html` and `head` start-tag recovery that merges missing
  attributes without nesting shell elements
- ignored late `head` start tags after body content has already started, with a
  parser diagnostic
- ignored self-closing flags on non-void HTML start tags, with parser
  diagnostics, so `<div/>`, `<script/>`, and `<td/>` still behave like open
  elements in HTML
- acknowledged self-closing syntax on void start tags plus ignored void end tags
  such as `</img>` and `</input>`, with parser diagnostics
- implied table `tbody` and `tr` creation for common omitted table structure
- implied table `colgroup` creation for bare `col` elements
- table caption/column-group boundary recovery before bare columns, rows, and
  sections
- parser-controlled lexer handoff for `title`, `textarea`, RAWTEXT elements,
  `script`, and `plaintext`
- parser options for scripting-sensitive tokenizer handoff, including
  `noscript`
- parser-approved initial tokenizer contexts for data-state documents and
  RCDATA/RAWTEXT, PLAINTEXT, foreign-content CDATA, comment, DOCTYPE,
  script-state, and intermediate tokenizer fragments exposed by the lexer,
  including resumable end-tag-open, seeded end-tag continuation, seeded comment
  continuation, seeded text/RCDATA character-reference continuation, and seeded
  DOCTYPE continuation contexts
- simple implied end tags for `p`, `li`, `dt`, `dd`, `option`, `optgroup`,
  ruby annotations, heading elements, legacy paragraph/block boundaries, and
  raw-text block starts
- scope-aware omitted-end recovery for those implied-end-tag families even when
  nested inline descendants are still open
- scope-aware table caption, column group, row group, row, and cell boundary
  recovery across nested inline descendants
- interactive and form-boundary recovery for repeated `a`, `button`, and
  `nobr` starts plus ignored nested `form` starts while preserving surrounding
  paragraph/list context
- special end-tag recovery for `</p>` and `</br>` compatibility cases
- omitted shell end-tag recovery for common `</head>`, `</body>`, and `</html>`
  boundaries in documents that rely on implied wrapper elements
- initial line-feed stripping for `pre`, `listing`, and `textarea`
- parser diagnostics for unmatched end tags
- body-fragment parsing that returns DOM nodes without the implied
  `html/head/body` shell while preserving lexer/parser diagnostics

The checked-in html5lib tree-construction smoke corpus now covers every case in
the currently audited upstream `html5lib-tests/tree-construction/*.dat` sources
by source signature. A separate adapter can project DOM into `document-ast` for
existing native document rendering.

## Conformance Audit

The tree-construction fixture lives in
`tests/fixtures/html5lib-tree-construction-smoke.dat`, and the shared
html5lib tokenizer corpus lives under `../html-lexer/tests/fixtures`. To verify
that the checked-in fixtures still cover an upstream html5lib-tests checkout:

```bash
HTML5LIB_TESTS_ROOT=/path/to/html5lib-tests \
  python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py
```

The audit fails if an upstream tree-construction case is missing, if an
upstream tokenizer case is missing from the raw mirrored tokenizer corpus, or
if the normalized tokenizer corpus records skipped cases.

## Usage

```rust
use coding_adventures_html_lexer::HtmlScriptingMode;
use coding_adventures_html_parser::{
    parse_html, parse_html_fragment, parse_html_with_options, HtmlInitialTokenizerContext,
    HtmlParseOptions,
};
use dom_core::Node;

let document = parse_html("<p>Hello <strong>Venture</strong></p>").unwrap();

match &document.children[0] {
    Node::Element(element) => assert_eq!(element.name, "html"),
    other => panic!("expected element, got {other:?}"),
}

let fragment_nodes = parse_html_fragment("<p>One<p>Two").unwrap();
assert_eq!(fragment_nodes.len(), 2);

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

let comment_fragment = parse_html_with_options(
    " body --><p>done</p>",
    HtmlParseOptions {
        initial_tokenizer_context: HtmlInitialTokenizerContext::Comment,
        ..HtmlParseOptions::default()
    },
)
.unwrap();

let doctype_fragment = parse_html_with_options(
    "ml><p>done</p>",
    HtmlParseOptions {
        initial_tokenizer_context: HtmlInitialTokenizerContext::DoctypeName,
        ..HtmlParseOptions::default()
    },
)
.unwrap();
```
