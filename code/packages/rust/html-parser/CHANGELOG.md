# Changelog

All notable changes to the `coding-adventures-html-parser` crate will be
documented in this file.

## [0.1.0] - 2026-05-02

### Added
- Initial HTML parser crate that consumes `coding-adventures-html-lexer` tokens
  and builds a `dom-core` document.
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
- Parser-approved initial tokenizer contexts now include seeded RCDATA, RAWTEXT,
  script data, and escaped script end-tag continuation substates, carrying the
  current end tag and temporary buffer required by those lexer states.
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
- `</p>` end tags without an open paragraph now create and close an implied
  empty paragraph with a parser diagnostic, matching common browser recovery.
- `</br>` now recovers as a `br` start tag with a parser diagnostic.
- `pre`, `listing`, and `textarea` now strip one immediately following LF text
  character while preserving later nested text.
- Omitted-shell `</head>`, `</body>`, and `</html>` boundaries now recover
  without noisy unmatched-end diagnostics by closing the current lightweight
  body-content stack before subsequent text or element siblings are appended.
- Implied end-tag recovery is now scope-aware for paragraphs, list items,
  definition items, select options, headings, ruby annotations, and table
  caption/column/row/cell contexts, so omitted-end boundaries close correctly
  even when nested inline descendants are still open.
