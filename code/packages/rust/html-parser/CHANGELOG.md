# Changelog

All notable changes to the `coding-adventures-html-parser` crate will be
documented in this file.

## Unreleased

### Added
- Stray paragraph end tags before the parser has implied the document body now
  reuse the existing pre-body diagnostic, closing 4 previously silent malformed
  corpus cases without changing DOM recovery.
- The after-after-frameset insertion mode now reports unexpected character,
  start-tag, and end-tag tokens, closing 8 previously silent malformed corpus
  cases without changing DOM recovery.
- An accepted `</body>` token now enters the after-body insertion mode even
  when `html`, `head`, and `body` were all implied, closing 5 previously silent
  malformed corpus cases for following head-only and frameset start tags.
- Full-document parsing now reports unexpected tokens that force recovery from
  the Standard's after-body and after-after-body insertion modes, closing 8
  previously silent malformed corpus cases without changing DOM recovery.
- Initial doctypes now report the Standard's parse error when their name,
  public identifier, or system identifier does not match the allowed HTML
  forms, closing 14 previously silent malformed corpus cases.
- Full-document parsing now reports the in-body EOF parse error when disallowed
  elements remain on the stack, closing 335 previously silent malformed corpus
  cases without changing DOM recovery or fragment diagnostics.
- `body` and `html` end tags now report the Standard's parse error when
  disallowed elements remain on the stack of open elements, closing 7
  previously silent malformed corpus cases without changing DOM recovery.
- Duplicate `html`, `head`, and `body` start tags now emit tree-construction
  diagnostics while retaining the Standard's attribute-merge recovery for
  `html` and `body`, closing 28 previously silent malformed corpus cases.
- Full-document parsing now emits a `missing-doctype` tree-construction
  diagnostic when the initial insertion mode sees a non-whitespace,
  non-comment, non-processing-instruction token before a doctype, closing 783
  previously silent malformed corpus cases.
- Tree-construction conformance cases now retain their declared parse-error
  rows and ratchet diagnostic coverage, exposing 1,577 malformed cases that
  still produce no parser or lexer diagnostic without weakening DOM checks.
- The conformance coverage report now pins the exact upstream WPT and
  html5lib-tests commits, and the package owns an explicit prioritized backlog
  and zero-debt completion boundary for future upstream drift.
- Browser render-tree extraction can now accept the fetched document URL as
  the fallback base, including resolution of relative authored `<base href>`
  values, so Venture can fetch relative links and inline images after HTTP
  navigation.
- The declarative HTML lexer now implements the current processing-instruction
  states, and the canonical 2,637-case tree corpus covers every source
  signature in the current 1,934-case upstream WPT tree-construction corpus.
- Processing instructions now have first-class tokenizer and DOM plumbing from
  declarative target/data emission through tree construction.
- The current WPT void-in-phrasing and foreign-content CDATA cases are now
  mirrored in the canonical tree-construction corpus and its focused void and
  foreign-content audits.
- The conformance coverage audit now reads tree-construction cases from WPT and
  tokenizer cases from html5lib-tests, pins every missing WPT source signature,
  and accepts only explicit missing-case debt counts so the now-zero gap cannot
  silently regress as upstream evolves.
- Fostered `nobr` table-cell continuation nodes are now repaired while
  processing the EOF token, retiring the final `finish_document` post-parse
  shim and the now-obsolete focused post-parse repair audit while preserving
  the rows in their table, formatting, shell, and interaction audits.
- The insanely badly nested html5lib table sequence is now repaired while
  processing the EOF token, retiring one finish-time post-parse shim while
  preserving its focused table and formatting audit coverage.
- `<hr>` inside open `select` elements is now placed during tree construction,
  removing a post-parse repair while preserving the html5lib select-list DOM
  audit behavior.
- Browser-readiness completion is now documented as a bounded planning surface
  and enforced by a manifest-style regression test that ties every public
  readiness inventory to fixture or focused-test evidence.
- Browser-readiness form-association descriptors now expose form owners, labels,
  fieldset membership, datalist links, and output calculation relationships as a
  flat browser-planning inventory.
- Browser-readiness form-reset descriptors now expose resettable controls,
  resetter controls, default reset values, selection/checked reset state, and
  form-level `onreset` hooks as a flat browser-planning inventory.
- Browser-readiness form-submission descriptors now expose successful controls,
  submission values, form action/method/target defaults, and submitter routing
  overrides as a flat browser-planning inventory.
- Browser-readiness form-validation descriptors now expose validation
  candidates, constraint attributes, barred controls, form-level `novalidate`,
  and submitter-level validation bypass hints as a flat browser-planning
  inventory.
- Browser-readiness context-menu interaction descriptors now expose
  `oncontextmenu` hooks, ARIA menu-popup invokers, menu/menuitem roles, popover
  menu surfaces, and hidden/inert/disabled blockers as a flat browser-planning
  inventory.
- Browser-readiness fullscreen-interaction descriptors now expose embedded
  fullscreen permission hints, `allowfullscreen` state, fullscreen event hooks,
  and document/body fullscreen callbacks as a flat browser-planning inventory.
- Browser-readiness animation-interaction descriptors now expose CSS animation
  and transition event hooks, timeline phase grouping, document/body scope, and
  cancellation paths as a flat browser-planning inventory.
- Browser-readiness lifecycle-event descriptors now expose document/body load
  and unload hooks, visibility/history/network lifecycle handlers, and
  element-level error recovery as a flat browser-planning inventory.
- Browser-readiness composition-interaction descriptors now expose IME
  composition events, beforeinput/input hooks, text controls, editing hosts, and
  blocked composition paths as a flat browser-planning inventory.
- Browser-readiness scroll-interaction descriptors now expose `onscroll`,
  `onscrollend`, wheel/touch routing, ARIA scrollbar value state, and blocked
  scroll paths as a flat browser-planning inventory.
- Browser-readiness pointer-interaction descriptors now expose click, mouse,
  touch, pointer, wheel, and drag/drop handler routing plus command/editing
  context and blocked pointer paths as a flat browser-planning inventory.
- Browser-readiness selection-interaction descriptors now expose `onselect` and
  selection-change handlers, input hooks, editing hosts, text controls, and
  blocked selection paths as a flat browser-planning inventory.
- Browser-readiness clipboard-interaction descriptors now expose copy/cut/paste
  handlers, input hooks, editing hosts, text controls, and blocked clipboard
  paths as a flat browser-planning inventory.
- Browser-readiness drag/drop descriptors now expose draggable state, drag/drop
  handlers, pointer handler inventory, and blocked drag paths as a flat
  browser-planning inventory.
- Browser-readiness input-planning descriptors now expose text-entry hints,
  datalist suggestions, validation blockers, form ownership, and contenteditable
  editing hosts as a flat browser-planning inventory.
- Browser-readiness keyboard-interaction descriptors now expose access keys,
  ARIA shortcuts, keyboard handlers, focus order, editing hosts, and blocked
  keyboard paths as a flat browser-planning inventory.
- Browser-readiness focus-navigation descriptors now expose sequential focus,
  programmatic focus, editing hosts, access keys, and hidden/inert/disabled/ARIA
  focus blockers as a flat browser-planning inventory.
- Browser-readiness activation descriptors now expose command, popover,
  disclosure, ARIA, focus, and inline handler routing metadata as a flat
  activation-planning inventory.
- Browser-readiness disclosure-state descriptors now expose details/dialog open
  state, grouped details names, summary text, modal/closedby behavior, and
  accessible naming metadata as a flat browser-planning inventory.
- Browser-readiness resource-endpoint descriptors now expose refresh redirects
  plus resolved document resources as a flat endpoint inventory for
  fetch/navigation planning.
- Browser-readiness event-handler descriptors now expose document, body, and
  element inline handler metadata as a categorized flat inventory for browser
  readiness planning without evaluating script text.
- Browser-readiness navigation-target descriptors now expose policy-rich anchor
  and image-map area targets with resolved URLs, target selection, rel policy,
  ping/attribution endpoints, language/type hints, referrer policy, download,
  and area geometry metadata as a flat browser-planning inventory.
- Browser-readiness document-policy descriptors now expose charset, viewport,
  referrer/robots/color-scheme policy, CSP/Permissions/Origin-Trial/Accept-CH
  hints, theme colors, refresh, canonical, and manifest metadata as a flat
  browser-planning inventory.
- Browser-readiness form-policy descriptors now expose form submission targets,
  accept/autocomplete/rel policy tokens, validation bypass state, and submitter
  overrides as a flat browser-planning inventory.
- Browser-readiness form-autofill descriptors now expose autocomplete tokens,
  section/address/contact field hints, webauthn markers, and disabled/readonly/
  hidden/off blockers as a flat browser-planning inventory.
- Browser-readiness fetch-policy descriptors now expose subresource integrity,
  CORS, nonce, referrer policy, iframe CSP/sandbox/permissions, fullscreen, and
  credentialless metadata as a flat browser-planning inventory.
- Browser-readiness loading-hint descriptors now expose lazy/eager loading,
  decoding, fetch priority, blocking, and media preload scheduling metadata.
- Browser-readiness ARIA relation descriptors now expose details, error-message,
  and flow target metadata with resolved target text.
- Browser-readiness global-state descriptors now include shell-level `html` and
  `body` states such as hidden, inert, accesskey, editing, spellcheck, and
  translate metadata.
- Browser-readiness text semantic summaries now include phrase-level semantics
  for abbreviation, definition, citation, code, keyboard input, sample output,
  variables, subscript/superscript, emphasis, importance, small print,
  strikethrough, and unarticulated annotation elements.
- Browser-readiness document summaries now include global-state descriptors for
  non-form elements with inert/hidden, editing, drag, spellcheck, translate,
  accesskey, autofocus, and related focus metadata.

### Fixed
- Residual HTML start tags processed through the table insertion mode now use
  foster parenting after table-context and head-element exceptions, keeping a
  `marquee` sibling before its open table in the current adoption-agency WPT.
- Replacement characters from null input are now stripped only at MathML and
  SVG HTML integration points, preserving them in ordinary foreign-content
  text while matching the current WPT fragment and CDATA cases.
- Test fixture generators: `COMMENT_MARKUP` regex accepts both `-->` and
  `--!>` comment end forms, and tag-axis classifier scans pass `re.I` even
  though their inputs are already lowercased.  Both changes silence codeql
  `py/bad-tag-filter`; runtime behaviour is unchanged.

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
- Duplicate open `html` start tags merge missing attributes into the existing
  document element, while duplicate `head` start tags are ignored instead of
  creating nested shell DOMs.
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
- Browser-facing resource summaries now carry link/resource scheduling
  metadata for preconnect, preload, modulepreload, prefetch, manifest,
  canonical, and icon links, including `as`, integrity, CORS/referrer policy,
  fetch priority, blocking, and responsive image preload hints.
- Browser-facing document, content-tree, and render-tree projections now carry
  identity and language metadata, including document/body `lang` and `dir`,
  body `id`/classes, and node-level `id`, tokenized `class`, `title`, `lang`,
  and `dir` values for selector matching and browser UI policy.
- Browser-facing document, content-tree, and render-tree projections now carry
  embedded resource metadata for frames, objects, embeds, media, and images,
  including resolved source URLs, resource kind, type hints, media attributes,
  authored dimensions, and flattened embedded policy descriptors for fetch,
  sandboxing, permissions, and fallback planning.
- Browser-facing document, content-tree, and render-tree projections now carry
  media playback metadata for `audio` and `video`, including playback flags,
  preload/poster fields, and flattened playback descriptors with source/track
  counts.
- Browser-facing document summaries now carry script and stylesheet loading
  metadata, including script kind, async/defer/nomodule flags, inline
  script/style text, integrity/crossorigin/referrer-policy hints, fetch
  priority, blocking, flattened script execution and stylesheet planning
  descriptors, alternate stylesheets, and disabled stylesheet state.
- Browser-facing image summaries now carry responsive image selection metadata,
  including `srcset`/`sizes`, resolved candidate URLs, flattened image
  candidate descriptors with source/candidate counts, `picture/source`
  media/type hints, lazy loading, decoding, fetch priority, CORS/referrer
  policy, usemap, and server-side image-map state.
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
