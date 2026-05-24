# Changelog

All notable changes to the `coding-adventures-html-parser` crate will be
documented in this file.

## [0.1.0] - 2026-05-02

### Added
- Initial HTML parser crate that consumes `coding-adventures-html-lexer` tokens
  and builds a `dom-core` document.
- Generated WHATWG tree-insertion audit fixture over the html5lib
  adoption-agency, table/foster-parenting, template, foreign-content fragment,
  and HTML fragment-shell families, with a dedicated parser regression test.
- Generated WHATWG frameset audit fixture over html5lib frameset, frame,
  noframes, foreign-content, body-compatibility, and template-boundary cases,
  with a dedicated parser regression test.
- Generated WHATWG table audit fixture over html5lib table shells, row groups,
  cells, captions/colgroups, foster-parenting, select-in-table recovery, and
  table fragment contexts, with a dedicated parser regression test.
- Generated WHATWG form/interactive audit fixture over html5lib anchor/nobr,
  button, form-control, select/option, textarea, fragment-context, and stray
  interactive end-tag cases, with a dedicated parser regression test.
- Generated WHATWG text-control audit fixture over html5lib script RAWTEXT,
  title/textarea RCDATA, RAWTEXT elements, noscript, PLAINTEXT, pre/listing,
  fragment-context, and stray text-control end-tag cases, with a dedicated
  parser regression test.
- Generated WHATWG foreign-content audit fixture over html5lib SVG, MathML,
  foreign fragment, HTML integration point, and table/foreign-boundary cases,
  with a dedicated parser regression test.
- Generated WHATWG formatting/implied-end-tag audit fixture over html5lib
  adoption-agency, anchor/nobr, paragraph, list/definition, ruby, heading, and
  formatting reconstruction cases, with a dedicated parser regression test.
- Generated WHATWG ruby audit fixture over html5lib `ruby`, `rb`, `rt`,
  `rtc`, and `rp` implied-end-tag recovery cases, including block descendants
  and nested ruby containers, with a dedicated parser regression test.
- Generated WHATWG noscript audit fixture over html5lib scripting-on/off,
  head-insertion, comment-boundary, text-mode-descendant, paragraph, and stray
  end-tag cases, with a dedicated parser regression test.
- Generated WHATWG head/body audit fixture over html5lib shell transitions,
  head metadata relocation, head text-mode handoff, frameset/body
  compatibility, and late shell tags, with a dedicated parser regression test.
- Generated WHATWG void-element audit fixture over html5lib void insertion,
  stray void end tags, table/select contexts, fragment contexts, foreign
  boundaries, and legacy void-like element cases, with a dedicated parser
  regression test.
- Generated WHATWG list-item audit fixture over html5lib `li`, `dt`, and `dd`
  implied-end-tag recovery, nested list boundaries, paragraph/list
  interactions, formatting reconstruction, and list-in-table recovery cases,
  with a dedicated parser regression test.
- Generated WHATWG paragraph audit fixture over html5lib paragraph implied-end
  tags, formatting reconstruction, table/foster-parenting boundaries, form
  controls, text modes, headings, special end-tag recovery, and fragment
  contexts, with a dedicated parser regression test.
- Generated WHATWG block-boundary audit fixture over html5lib grouping,
  sectioning, list-container, heading, formatting, table, foreign-content,
  template, form, text-mode, ruby, select/list, and fragment-context cases,
  with a dedicated parser regression test.
- Generated WHATWG fragment-context audit fixture over html5lib table, shell,
  block, foreign-content, text-mode, select/list, template, and ordinary
  fragment parsing contexts, with a dedicated parser regression test.
- Generated WHATWG character-reference audit fixture over html5lib named,
  numeric, ambiguous ampersand, attribute, RCDATA, and fragment-context
  character-reference cases, with a dedicated parser regression test.
- Generated WHATWG legacy/edge element audit fixture over html5lib `isindex`,
  obsolete `menuitem`, `main`/`search`, pending-spec, tricky recovery, and
  namespace-sensitivity cases, with a dedicated parser regression test.
- Generated WHATWG document-shell audit fixture over html5lib doctype, comment,
  html/head/body synthesis, frameset-boundary, and shell fragment-context
  cases, with a dedicated parser regression test.
- Generated WHATWG template audit fixture over html5lib template shell,
  table/select/frameset interactions, nested-template, EOF, document-shell,
  text-mode, foreign-content, and fragment-context cases, with a dedicated
  parser regression test.
- Generated WHATWG select/list audit fixture over html5lib select shell,
  option implied-end, optgroup-boundary, select-in-table, fragment-context, and
  stray select/list end-tag cases, with a dedicated parser regression test.
- Generated WHATWG miscellaneous recovery audit fixture over html5lib
  XML/processing-instruction-looking markup, bogus comments, CDATA-as-text,
  malformed tag opens, plain text and whitespace shells, duplicate doctypes,
  unknown/custom elements, and legacy compatibility cases, with a dedicated
  parser regression test.
- The miscellaneous recovery audit now covers the residual `tests1`,
  `tests19`, and `tests25` source-marker variants, and a focused audit
  coverage check verifies that every checked tree-construction smoke case is
  indexed by at least one `whatwg-*-audit.json` parser fixture.
- Shared html5lib tree-construction test helpers keep smoke and focused
  tree-insertion, frameset, table, form/interactive, text-control,
  foreign-content, formatting, ruby, noscript, head/body, document-shell,
  void-element, list-item, paragraph, block-boundary, fragment-context,
  character-reference, legacy/edge element, template, and select/list audit
  checks on the same parser and DOM dump path.
- The html5lib source-coverage audit now has a checked JSON report plus
  `--write-report` and `--check-report` modes for parser/tokenizer fixture
  drift detection.
- A parser integration test verifies that the checked coverage-audit report
  still matches the checked tree-construction, raw tokenizer, and normalized
  tokenizer fixture corpora.
- The shared generated HTML fixture manifest check can include parser coverage
  report and pinned-count checks alongside the self-contained lexer/parser
  fixture stale checks.
- Stack-of-open-elements tree construction seed with void element handling,
  adjacent text merging, simple implied end tags, and unmatched end-tag
  diagnostics.
- Parser-driven tokenizer handoff for RCDATA, RAWTEXT, script data, and
  PLAINTEXT elements, preserving text-mode DOM content instead of lexing it as
  ordinary data-state markup.
- Implied `html`, `head`, and `body` document shell normalization, including
  preservation of explicit shell attributes and legacy omitted-wrapper pages.
- Explicit `head` elements now close before `body` starts or non-head body
  content appears, preventing omitted `</head>` pages from trapping body DOM
  inside the head.
- Duplicate open `body` start tags now merge missing attributes into the
  existing body instead of creating nested body elements.
- Scripting-aware parse options for parser-controlled tokenizer handoff, so
  `noscript` becomes RAWTEXT with scripting enabled and ordinary fallback
  markup with scripting disabled.
- Parser-approved initial tokenizer contexts, including foreign-content CDATA
  section fragments backed by the typed lexer CDATA context.
- Parser-approved initial script tokenizer contexts for script data, escaped,
  dash/dash-dash, less-than, and double-escaped substates backed by the typed
  lexer script-substate context helper.
- Parser-approved initial tokenizer contexts now cover RCDATA/RAWTEXT
  fragments, CDATA bracket/end substates, script less-than and escape-start
  substates, and script double-escape start/end substates exposed by the lexer.
- Parser-approved initial tokenizer contexts now include RCDATA, RAWTEXT,
  script data, and escaped script end-tag-open substates exposed by the lexer,
  keeping parser fragment handoff aligned with the broader tokenizer surface.
- Parser-approved initial tokenizer contexts now include PLAINTEXT fragments,
  allowing parser-approved resumptions that consume all remaining input as text
  through EOF.
- Parser-approved initial tokenizer contexts now include seeded RCDATA, RAWTEXT,
  script data, and escaped script end-tag continuation substates, carrying the
  current end tag and temporary buffer required by those lexer states.
- Parser-approved initial tokenizer contexts now include seeded HTML comment
  continuation substates, carrying current comment data through comment body,
  pending dash/bang, nested-comment, abrupt-close, and bogus-comment recovery
  paths exposed by the lexer.
- Parser-approved initial tokenizer contexts now include seeded text/RCDATA
  character-reference continuation substates, carrying temporary buffers and
  return states through named, numeric, decimal, and hexadecimal recovery paths.
- Browser-facing content and render projections now expose document outline
  metadata for heading levels, sectioning elements, and landmark-like regions
  such as `main`, `nav`, `aside`, `header`, and `footer`.
- Initial table tree-construction recovery for omitted `tbody`/`tr` structure,
  including implicit row groups for bare rows/cells and section closure when a
  new table section starts.
- Table caption and column-group boundary recovery so captions/colgroups close
  before following rows, cells, and table sections in the lightweight DOM tree
  builder.
- Implied `colgroup` creation for bare `col` elements under tables, keeping
  column metadata grouped before following row sections.
- Caption boundary recovery before bare `col` elements, so captions close
  before implied column groups are created.
- Simple implied end-tag recovery for adjacent `option` and `optgroup`
  elements, preventing nested select-option DOMs when end tags are omitted.
- Heading start tags now close open paragraphs and previous headings, avoiding
  nested heading DOMs when heading end tags are omitted.
- Common block starts such as `div`, `ul`, and `table` now close open
  paragraphs before insertion, preventing paragraph-nested block DOMs.
- Ruby annotation starts now close omitted `rb`, `rt`, `rp`, and `rtc` siblings,
  preventing nested ruby annotation DOMs when end tags are omitted.
- Repeated interactive formatting starts for `a`, `button`, and `nobr` now
  close the previous open element before inserting the next one, avoiding
  impossible nested interactive DOMs for common omitted-end-tag markup.
- Repeated interactive starts now preserve the surrounding paragraph context
  when they recover, so trailing text and later inline siblings stay under the
  same paragraph instead of spilling to the body.
- Paragraph boundary recovery now covers additional legacy and modern block
  starts, including `button`, `center`, `dir`, `hgroup`, `search`, `listing`,
  `xmp`, and `plaintext`.
- Raw-text and plaintext block starts now close an open paragraph before
  tokenizer handoff, keeping the resulting text-mode elements as paragraph
  siblings.
- Nested `form` start tags are now ignored with a parser diagnostic while an
  outer form remains open, keeping form-associated content in the existing form
  instead of creating nested form DOMs.
- Duplicate open `html` and `head` start tags now merge missing attributes into
  the existing shell elements instead of creating nested shell DOMs.
- Late `head` start tags after body content has already started are ignored
  with a parser diagnostic.
- Self-closing flags on non-void HTML start tags are now ignored with a parser
  diagnostic, keeping elements such as `div`, `script`, `textarea`, and table
  cells open for their real content.
- Browser-facing document extraction now includes base targets, head metadata,
  loadable resources, anchor targets, richer link attributes, form encodings,
  form targets, and basic control disabled/checked state for browser pipeline
  consumers.
- Browser-facing content tree extraction now projects parsed body content into
  CSS-independent structural nodes for early browser rendering pipelines.
- Browser-facing render tree input extraction now maps parsed body structure
  into stable default display categories for early layout pipelines.
- Browser-facing document, content-tree, and render-tree projections now carry
  richer form control metadata including values, checked/selected state,
  disabled state, select options, and textarea values.
- Browser-facing document, content-tree, and render-tree projections now carry
  form accessibility metadata including explicit and implicit labels, derived
  accessible names, form-owner references, placeholder/autocomplete hints, and
  required/readonly/multiple control state.
- Browser-facing document, content-tree, and render-tree projections now carry
  resolved URL metadata for links, resources, images, and form actions using
  the document `base` href when available, while preserving raw authored
  attributes for downstream policy decisions.
- Browser-facing document, content-tree, and render-tree projections now carry
  identity and language metadata, including document/body `lang` and `dir`,
  body `id`/classes, and node-level `id`, tokenized `class`, `title`, `lang`,
  and `dir` values for selector matching and browser UI policy.
- Browser-facing document, content-tree, and render-tree projections now carry
  embedded resource metadata for frames, objects, embeds, media, and images,
  including resolved source URLs, resource kind, type hints, media attributes,
  and authored dimensions for fetch and layout planning.
- Browser-facing table summaries and tree projections now carry table layout and
  accessibility metadata, including effective column counts, column hints,
  row-group identity, column and cell spans, header associations, scopes, and
  abbreviated header labels.
- Browser-facing content-tree and render-tree projections now carry text-flow
  metadata for paragraphs, preformatted text, ordered and unordered lists, list
  item values, block and inline quote citations, and line/word/thematic break
  elements.
- Void end tags such as `</img>`, `</input>`, and `</hr>` are now ignored with a
  parser diagnostic, while self-closing syntax on void start tags remains
  acknowledged.
- `</p>` end tags without an open paragraph now create and close an implied
  empty paragraph with a parser diagnostic, matching common browser recovery.
- `</br>` now recovers as a `br` start tag with a parser diagnostic.
- `pre`, `listing`, and `textarea` now strip one immediately following LF text
  character while preserving later nested text.
- Omitted-shell `</head>`, `</body>`, and `</html>` boundaries now recover
  without noisy unmatched-end diagnostics by closing the current lightweight
  body-content stack before subsequent text or element siblings are appended.
- Body-fragment parsing APIs now return DOM nodes without the implied document
  shell while preserving lexer/parser diagnostics and parser options.
- Implied end-tag recovery is now scope-aware for paragraphs, list items,
  definition items, select options, headings, ruby annotations, and table
  caption/column/row/cell contexts, so omitted-end boundaries close correctly
  even when nested inline descendants are still open.
