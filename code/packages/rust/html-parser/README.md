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
- browser-facing document extraction for title, base URL/target, head metadata,
  resource inventory, anchor targets, body text, headings, richer links, image
  attributes, form controls, and table summaries
- browser-facing content tree extraction that filters parser-only shell and
  invisible nodes into a CSS-independent body structure for early rendering
- browser-facing render tree input extraction that maps renderable content
  nodes into stable default display categories for early layout
- browser-facing form control metadata for input values, disabled/checked/
  selected state, select options, and textarea values across document,
  content-tree, and render-tree projections
- browser-facing form accessibility metadata for explicit and implicit labels,
  derived accessible names, form-owner references, placeholder/autocomplete
  hints, and required/readonly/multiple control state across document,
  content-tree, and render-tree projections
- browser-facing URL resolution metadata for links, loadable resources, images,
  and form actions using the document `base` href when available, plus
  `parse_browser_render_tree_with_document_url` for resolving relative browser
  content against the fetched navigation URL while keeping raw authored URLs
  available to browser policy code
- browser-facing link/resource scheduling metadata for preconnect, preload,
  modulepreload, prefetch, manifest, canonical, and icon links, including
  `as`, integrity, CORS/referrer policy, fetch priority, blocking, and
  responsive image preload hints
- browser-facing responsive image metadata for `img` and `picture/source`
  combinations, including raw/resolved `srcset`, `sizes`, source media/type
  hints, lazy loading, decoding, fetch priority, CORS/referrer policy, usemap,
  and server-side image-map state
- browser-facing identity and language metadata for document/body fields plus
  renderable content nodes, including `id`, tokenized `class`, `title`, `lang`,
  and `dir` values for selector matching, UI policy, and early layout
- browser-facing embedded resource metadata for replaced content such as
  `iframe`, `object`, `embed`, `audio`, and `video`, including resolved source
  URLs, resource kind, type hints, media attributes, and authored dimensions
- browser-facing media playback metadata for `audio` and `video`, including
  playback flags and preload/poster fields
- browser-facing script and stylesheet loading metadata, including
  module/classic script kind, async/defer/nomodule flags, inline script/style
  text, integrity/crossorigin/referrer-policy hints, fetch priority, blocking,
  alternate stylesheet, and disabled stylesheet state
- browser-facing table layout and accessibility metadata, including effective
  column counts, column hints, row-group identity, column spans, cell spans,
  header associations, scopes, and abbreviated header labels
- browser-facing text-flow metadata for paragraphs, preformatted text, ordered
  and unordered lists, list item values, block/inline quote citations, and
  line/word/thematic break elements
- browser-facing document outline metadata for heading levels, sectioning
  elements, and landmark-like regions such as `main`, `nav`, `aside`,
  `header`, and `footer`

## Browser-Readiness Completion Boundary

The browser-readiness pass is complete when `BrowserDocument` exposes stable,
flat planning inventories for the major browser subsystems that need parser
facts before CSS, layout, script execution, accessibility tree construction,
network scheduling, and activation planning. The completion boundary is not
"add every possible browser API"; it is a bounded parser-owned contract:

- document/head policy, URL, resource, script, stylesheet, fetch, and loading
  inventories
- form ownership, control, validation, submission, reset, autofill, and form
  policy inventories
- navigation targets, navigation groups, headings, landmarks, text semantics,
  text flow, structured data, table structure, and global state inventories
- ARIA name, description, relation, collection, range, and live-region
  inventories
- media, embedded content, responsive image, image-map, canvas, template, slot,
  custom element, component-hydration, and data-attribute inventories
- activation, popover, disclosure, focus, keyboard, input, drag/drop,
  clipboard, selection, composition, pointer, scroll, lifecycle, animation,
  fullscreen, context-menu, and event-handler inventories

Each public browser-readiness inventory must have either checked fixture
evidence in `tests/fixtures/html-browser-readiness.json` or an explicitly named
focused regression test in `tests/browser_readiness_test.rs`. The
`browser_readiness_completion_manifest_matches_public_surface` test is the
executable source of truth for that boundary and should fail if a future field
is added without coverage evidence.

The checked-in tree-construction smoke corpus contains 2,637 passing DOM cases.
It mirrors every source signature in the current 1,934-case upstream WPT
tree-construction corpus. A separate adapter can project DOM into
`document-ast` for existing native document rendering.

## Conformance Audit

The tree-construction fixture lives in
`tests/fixtures/html5lib-tree-construction-smoke.dat`, and the shared html5lib
tokenizer corpus lives under `../html-lexer/tests/fixtures`. Tree-construction
tests moved from html5lib-tests to WPT, so a current audit uses both checkouts:

```bash
HTML5LIB_TESTS_ROOT=/path/to/html5lib-tests \
WPT_ROOT=/path/to/wpt \
  python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py \
  --expect-tree-missing 0 \
  --expect-tokenizer-missing 0
```

Without explicit missing-case expectations, the audit fails if an upstream
tree-construction or tokenizer case is absent locally. Supplying an exact
missing-case expectation accepts only that checked debt count; the stable report
also pins every missing source signature so same-count churn remains visible.
Normalized tokenizer skips always fail the audit.

For CI jobs that need to catch accidental fixture drift as well as missing
coverage, the audit can pin the current corpus counts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py \
  /path/to/html5lib-tests \
  --wpt-root /path/to/wpt \
  --expect-tree-upstream-cases 1934 \
  --expect-tree-local-cases 2637 \
  --expect-tree-missing 0 \
  --expect-tokenizer-upstream-cases 6806 \
  --expect-tokenizer-local-raw-cases 7015 \
  --expect-tokenizer-missing 0 \
  --expect-normalized-cases 7242 \
  --expect-normalized-skipped 0
```

The audit also owns a checked `tests/fixtures/html5lib-coverage-audit.json`
summary. It records the exact WPT and html5lib-tests commits alongside corpus
counts and missing source signatures, so the evidence is reproducible even
when upstream changes without changing a case count. Regenerate or verify that
exact report with `--write-report` or `--check-report`. The Rust integration
tests also parse that report and compare its local corpus counts against the
checked tree-construction and tokenizer fixtures.

The prioritized completion queue and intake rules live in
`CONFORMANCE-BACKLOG.md`.

Tree-construction data also carries legacy `#errors` rows. The shared test
loader retains those rows and the main tree-construction test ratchets whether
each malformed case produces any lexer or parser diagnostic. Exact legacy
error strings are not treated as a public WHATWG taxonomy; they are used to
identify missing diagnostic coverage while DOM output remains independently
checked. Full-document parsing follows the Standard's initial insertion mode:
whitespace, comments, and processing instructions may precede a doctype, while
any other token first emits `missing-doctype` and enters quirks handling.
Repeated `html`, `head`, and `body` start tags also emit shell diagnostics;
`html` and `body` retain the Standard's missing-attribute merge behavior while
a repeated `head` token is ignored.

For sharper parser regression reporting, the generated
`tests/fixtures/whatwg-tree-insertion-audit.json` fixture indexes the
adoption-agency, table/foster-parenting, template, foreign-content fragment,
and HTML fragment-shell cases inside the tree-construction smoke corpus:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_tree_insertion_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-frameset-audit.json` fixture similarly
indexes frameset, frame, noframes, foreign-content, body-compatibility, and
template-boundary recovery cases:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_frameset_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-table-audit.json` fixture splits the
table-construction coverage into table shells, row groups, cells,
captions/colgroups, foster-parenting, select-in-table recovery, and table
fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_table_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-form-interactive-audit.json` fixture
indexes anchor/nobr recovery, button boundaries, form-associated controls,
select/option handling, textarea RCDATA handoff, interactive fragment contexts,
and stray interactive end tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_form_interactive_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-text-control-audit.json` fixture indexes
script RAWTEXT, title/textarea RCDATA, RAWTEXT elements, noscript scripting
modes, PLAINTEXT, pre/listing initial newlines, text-control fragment contexts,
and stray text-control end tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_text_control_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-foreign-audit.json` fixture indexes SVG,
MathML, foreign-content fragments, HTML integration points, and
table/foreign-content boundaries:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_foreign_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-formatting-audit.json` fixture indexes
active formatting elements, adoption-agency recovery, paragraph/list implied
end tags, ruby scopes, headings, and formatting reconstruction:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_formatting_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-ruby-audit.json` fixture indexes `ruby`,
`rb`, `rt`, `rtc`, and `rp` implied-end-tag recovery, including block
descendants and nested ruby containers:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_ruby_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-noscript-audit.json` fixture indexes
`noscript` behavior with scripting enabled and disabled, head insertion-mode
boundaries, comment-looking text, RAWTEXT/PLAINTEXT descendants, paragraph
integration, and stray noscript end tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_noscript_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-head-body-audit.json` fixture indexes
`html`/`head`/`body` shell transitions, head metadata relocation, title/style/
script handoff, frameset/body compatibility, and late shell tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_head_body_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-void-element-audit.json` fixture indexes
void element insertion, stray void end tags, table/select contexts, fragment
contexts, foreign-content boundaries, and legacy void-like elements:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_void_element_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-list-item-audit.json` fixture indexes
`li`, `dt`, and `dd` implied-end-tag recovery, nested list boundaries,
paragraph/list interactions, formatting reconstruction, and list-in-table
recovery:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_list_item_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-paragraph-audit.json` fixture indexes
paragraph implied-end tags, formatting reconstruction, table/foster-parenting
boundaries, form controls, text modes, headings, special end-tag recovery, and
fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_paragraph_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-block-boundary-audit.json` fixture indexes
grouping, sectioning, list-container, heading, formatting, table,
foreign-content, template, form, text-mode, ruby, select/list, and fragment
context block boundaries:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_block_boundary_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-fragment-context-audit.json` fixture
indexes table, shell, block, foreign-content, text-mode, select/list, template,
and ordinary fragment parsing contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_fragment_context_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-character-reference-audit.json` fixture
indexes named references, numeric references, ambiguous ampersands, attribute
references, parser-driven RCDATA references, and fragment-context
character-reference handling:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_character_reference_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-legacy-element-audit.json` fixture
indexes `isindex`, obsolete `menuitem`, `main`/`search`, pending-spec, tricky
recovery, and namespace-sensitivity cases:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_legacy_element_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-document-shell-audit.json` fixture indexes
doctypes, comments, `html`/`head`/`body` synthesis, frameset boundaries, and
shell fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_document_shell_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-template-audit.json` fixture indexes
template insertion modes, nested template stacks, EOF recovery,
table/select/frameset interactions, document-shell placement, text-mode
content, foreign content, and template fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_template_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-select-list-audit.json` fixture indexes
select shells, option implied-end recovery, optgroup boundaries,
select-in-table handling, select fragment contexts, and stray select/list end
tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_select_list_audit_fixture.py \
  --check
```

The generated `tests/fixtures/whatwg-processing-instruction-audit.json`
fixture indexes all current WPT processing-instruction tree cases across
target validation, data normalization, EOF recovery, and insertion contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_processing_instruction_audit_fixture.py \
  --check
```

The broad smoke test and the focused audit fixtures use the shared
`tests/common` html5lib parser and DOM dump helpers, so new parser conformance
fixtures exercise the same normalization path.

The focused parser audit coverage guard verifies that every source marker in
the checked tree-construction smoke fixture is indexed by at least one
`whatwg-*-audit.json` fixture:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/check_whatwg_audit_coverage.py \
  --check
```

The lexer fixture directory also provides one umbrella stale check for all
self-contained generated HTML lexer/parser fixtures:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/check_generated_html_fixtures.py
```

Pass `--html5lib-tests /path/to/html5lib-tests --wpt-tests /path/to/wpt` to
fold the checked coverage audit report and pinned-count checks into the same
local guard.

## Usage

```rust
use coding_adventures_html_lexer::HtmlScriptingMode;
use coding_adventures_html_parser::{
    parse_browser_content_tree, parse_browser_document, parse_browser_render_tree, parse_html,
    parse_html_fragment, parse_html_with_options, HtmlInitialTokenizerContext, HtmlParseOptions,
};
use dom_core::Node;

let document = parse_html("<p>Hello <strong>Venture</strong></p>").unwrap();
let browser_document = parse_browser_document(
    "<title>Example</title><h1>Example</h1><p><a href=next.html>Next</a></p>"
).unwrap();

assert_eq!(browser_document.title.as_deref(), Some("Example"));
assert_eq!(browser_document.links[0].href.as_deref(), Some("next.html"));
assert!(browser_document.resources.is_empty());

let content_tree = parse_browser_content_tree("<h1>Example</h1><p>Hello <b>there</b>").unwrap();
assert_eq!(content_tree.children[0].role, "heading");
assert_eq!(content_tree.children[1].role, "block");

let render_tree = parse_browser_render_tree("<p>Hello <img src=logo.gif alt=Logo>").unwrap();
assert_eq!(render_tree.children[0].display, "block");
assert_eq!(render_tree.children[0].children[1].display, "inline-replaced");

let form_tree = parse_browser_render_tree(
    "<form><select><option value=one>One<option selected>Two</select></form>"
).unwrap();
let select = &form_tree.children[0].children[0];
assert_eq!(select.control_type.as_deref(), Some("select"));
assert_eq!(select.value.as_deref(), Some("Two"));
assert_eq!(select.options, vec!["One".to_string(), "Two".to_string()]);

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
