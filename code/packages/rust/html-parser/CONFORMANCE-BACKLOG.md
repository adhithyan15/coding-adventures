# HTML Parser Conformance Backlog

Last audited: 2026-08-09

## Completion Boundary

The parser conformance loop is complete when a fresh audit against current WPT
tree-construction resources and html5lib tokenizer tests reports:

- zero missing WPT tree-construction signatures
- zero missing html5lib tokenizer signatures
- zero normalized tokenizer skips
- every tree-construction case with declared parse errors emits diagnostics
- tree-construction cases without declared parse errors do not gain spurious
  diagnostics unless the checked fixture intentionally omits a current WHATWG
  error
- all checked parser, lexer, fixture, formatting, and lint gates pass

The checked `tests/fixtures/html5lib-coverage-audit.json` report records the
exact upstream commits used for the latest completed audit.

## Prioritized Queue

The 2026-08-10 upstream audit at WPT
`f3d30c116e82b06f72831f27b7fe94a2f8114030` and html5lib-tests
`224991ec10db04f056a89eed8b0bd8695fd2950e` covered all 1,934 WPT
tree-construction cases and all 6,806 html5lib tokenizer cases with zero missing
signatures and zero normalized skips. DOM output is complete, but diagnostic
coverage is not:
the checked 2,637-case tree corpus declares 6,243 errors across 2,183 cases.
After the seeded-SVG foreign-breakout diagnostic slice, all 2,183 malformed
cases emit at least one lexer or parser diagnostic and none remain uncovered.
Another 139 cases emit diagnostics despite having no legacy `#errors` rows.
These are reviewed rather than automatically removed: 89 are full-document
inputs for which the legacy fixtures omit the Standard-required missing-doctype
error, including the current processing-instruction cases.

The concrete document-shell insertion-mode inventory is complete: missing and
nonconforming doctypes, duplicate shell tags, body/html boundary errors,
post-head head-content starts, after-body and frameset modes, implied-shell
paragraph end tags, and rejected frameset starts all report their
current-Standard parse errors.

Fragment in-body EOF diagnostics are also complete. Authored disallowed open
elements now report the same error as full-document parsing, while synthetic
context and table-wrapper nodes are excluded. Remaining fragment EOF-labeled
rows depend on table foster-parenting/scope recovery or foreign-content table
boundaries and stay assigned to those algorithm audits.

Text insertion-mode EOF diagnostics are complete for authored `script`,
RCDATA, RAWTEXT, and scripting-enabled `noscript` elements. EOF now reports the
mode's parse error, pops the current text element, and reprocesses EOF in the
original mode; synthetic text fragment contexts remain diagnostic-free.

The in-body "any other end tag" parse error is now reported when implied end
tags leave the matching open element non-current, including special-element
scope stops and nested formatting recovery. Remaining in-body work covers
formatting-reconstruction and other stray-tag branches. Obsolete `menuitem`
end tags now report when no matching element is current. Heading end tags report
the specialized parse error when no heading is in scope or the current heading
does not match the token. A `li` start tag reports when its implied-end-tag
recovery closes a non-current list item, and a paragraph end tag reports when it
breaks out of MathML foreign content. A formatting end tag also reports when an
open table blocks adoption-agency recovery. Foreign formatting end tags now
report when they do not match the current foreign node, close a matching foreign
element before the HTML boundary, re-enter adoption-agency recovery for a
matching HTML ancestor, and report the ordinary unmatched ending when neither
exists. Description-list item start tags now report when implied-end-tag
recovery closes a non-current `dt` or `dd`, without flagging adjacent
description-list items.

There are no residual malformed tree-construction cases without a lexer or
parser diagnostic. HTML `p` and `br` start tags recovered from seeded foreign
fragment contexts now report their foreign-content breakout parse error. A
second `body` end tag in a seeded `html` fragment reports its after-body
insertion-mode parse error, and non-whitespace text discarded by a seeded
`colgroup` context reports its table insertion-mode parse error.

Prioritized work items:

1. **Table, select, and template insertion modes.** Continue the algorithm
   audit beyond the currently exercised diagnostic corpus.
   Non-whitespace table text now reports when it is foster-parented. The
   specialized SVG and MathML start-tag path now reports when table insertion
   mode foster-parents foreign content, and table-context end tags now report
   when they force recovery from foreign content. Hidden `input` start tags in
   table structure now report their required parse error while retaining their
   special in-table insertion behavior. Foster-parented `title` and `meta`
   start tags now report the general in-table parse error. Nested `select` and
   `input` start tags now report when they force recovery from select mode.
   Repeated `option` and `optgroup` start tags now report the current-Standard
   parse error when implied-end-tag recovery finds an option or optgroup in
   select scope; the legacy html5lib error rows exercise the DOM recovery
   without declaring that branch error.
   Non-whitespace text after a template row now reports when it forces recovery
   from template table mode. Formatting start tags now report when table
   structure foster-parents them, covering the remaining silent fostered-anchor
   starts. Caption recovery now reports when a table-structure start tag or
   `table` end tag closes the caption while a non-implied node remains current;
   caption-scoped table endings now reprocess after closing the caption.
   Cell start tags now report when table-body recovery must synthesize a
   missing row, matching the current Standard and WPT's
   `unexpected-cell-in-table-body` evidence while leaving normal row and
   template-mode cells quiet.
   Nested `table` start tags now report before table-mode recovery closes the
   open table and reprocesses the token, matching the minimal WPT
   `tests6.dat` `<table><table>` evidence while leaving tables nested inside
   cells quiet.
   `form` start tags processed in table mode now report the required parse
   error both when the form pointer is initially null and when a repeated form
   is ignored, matching WPT `tests20.dat` `unexpected-form-in-table` evidence
   while leaving forms inside cells quiet.
   Non-hidden `input` start tags processed in table mode now report the general
   parse error before foster parenting, matching WPT `tests7.dat` and
   `webkit02.dat` evidence while preserving the specialized hidden-input path
   and leaving inputs inside cells quiet.
   `select` start tags processed directly in table structure now report the
   general in-table parse error before foster parenting, matching WPT
   `tests10.dat`, `tests17.dat`, `tests18.dat`, and `webkit01.dat` evidence while
   leaving selects outside tables and inside cells quiet.
   Generic paragraph-boundary, `br`, `p`, and `plaintext` start tags processed
   directly in table structure now report the general in-table parse error
   before foster parenting, matching WPT `tests8.dat`, `tricky01.dat`,
   `tests18.dat`, and `template.dat` evidence while leaving the same starts
   outside tables and inside cells quiet.
   `li` start tags processed in table foster-parenting state now report the
   general in-table parse error for both a direct table-structure start and a
   repeated start whose parent is already fostered before the table, matching
   WPT `tests8.dat` while leaving list items outside tables and inside cells
   quiet.
   `img` start tags processed directly in table structure now report the
   general in-table parse error and foster before the table, including the
   specialized center/font reconstruction path evidenced by WPT
   `tricky01.dat`, while leaving images outside tables and inside cells quiet.
   Generic `div` and `span` end tags processed while table foster parenting is
   active now report the general in-table parse error, matching WPT
   `tests8.dat` evidence while leaving the same endings outside tables and
   inside cells quiet.
   `center` end tags processed while table foster parenting is active now
   report the general in-table parse error alongside non-current end-tag
   recovery, matching WPT `tricky01.dat` while leaving centers outside tables
   and inside cells quiet.
   EOF reached while table structure or foster parenting remains active now
   reports the dedicated table-mode parse error instead of the broader
   unclosed-elements diagnostic, matching WPT's `eof-in-table` evidence while
   leaving closed tables, open cells, and closed table fragments on their
   existing paths.
   EOF reached with authored open templates now reports one template-mode
   parse error for each open template before evaluating the residual insertion
   mode, matching the focused `template.dat` EOF cases across ordinary,
   nested, table, select, and text-mode contexts. Closed templates and the
   synthetic template fragment context remain quiet.
   A `template` end tag with no authored HTML template open now reports the
   in-head parse error and remains ignored, matching WPT `template.dat` while
   preserving matching HTML-template closure, foreign template-named element
   closure, and the synthetic template-fragment boundary diagnostic.
   A matching HTML `template` end tag now thoroughly generates implied end
   tags and reports when a non-template element remains current, matching WPT
   `template.dat`'s mismatched-template evidence while leaving implied
   descendants, directly current templates, foreign template-named elements,
   and synthetic template fragment contexts quiet.
   Start tags rejected after a template-owned `col` now report the
   in-column-group parse error before remaining ignored, matching WPT
   `template.dat`'s `<template><col><div>` and `<template><col><colgroup>`
   evidence while leaving additional columns, nested templates, columns
   outside templates, and ordinary `colgroup` recovery on their existing
   paths. Non-whitespace character data rejected after a template-owned `col`
   now reports the same in-column-group boundary error while remaining ignored,
   matching WPT `template.dat`'s `<template><col>Hello` evidence. ASCII
   whitespace is retained, while ordinary content, real column groups and
   cells, nested templates, foreign template-named elements, and synthetic
   template fragment contexts keep their existing diagnostic behavior.
   `colgroup` and `col` end tags rejected after a template-owned `col` now
   report the in-column-group parse error while remaining ignored, matching
   WPT `template.dat`'s `<template><col></colgroup>` and template-owned
   `</col>` evidence. Matching real `colgroup` closure, `template` closure,
   ordinary in-body recovery, foreign template-named elements, and synthetic
   template fragment contexts remain on their existing diagnostic paths.
   Unrelated end tags rejected after a template-owned `col` now report the
   in-column-group anything-else parse error while remaining ignored, matching
   WPT `template.dat`'s `<template><col></div>` evidence. The dedicated
   `colgroup` and `col` endings retain the same diagnostic, while `template`
   closure, ordinary in-body recovery, real column groups, foreign content,
   and synthetic template fragment contexts remain on their existing paths.
   Table-section, caption, and column-group starts rejected after a
   template-owned row or cell now report the template table-mode parse error
   while remaining ignored, matching WPT `template.dat`'s
   `<template><tr></tr><tbody>`, `<template><td></td><tbody>`, caption, and
   column-group evidence. Real tables, valid template table-section sequences,
   nested templates, foreign content, and synthetic template fragments remain
   on their existing diagnostic paths. A `table` end tag rejected after a
   template-owned row or cell now reports the template table-mode parse error
   while remaining ignored, matching WPT `template.dat`'s
   `<template><tr></tr></table>` and `<template><td></td></table>` evidence.
   Matching real-table closure, nested templates, foreign content, ordinary
   stray end tags, and synthetic template fragments remain on their existing
   diagnostic paths. A `tr` end tag rejected after a template-owned cell now
   reports the in-row scope parse error while remaining ignored, matching WPT
   `template.dat`'s `<template><td></td></tr>` evidence. Matching real-row
   closure, nested templates, foreign content, ordinary stray end tags, and
   synthetic template fragments remain on their existing paths. `td` and `th`
   starts after a closed template-owned row now report the in-table-body parse
   error before preserving the existing implied-row recovery, matching WPT
   `template.dat`'s `<template><tr></tr><td>` and nested-template evidence.
   Direct template-owned cells, cells in an open row, real tables, foreign
   content, and synthetic template fragments remain on their existing paths.
   A `tr` start processed in ordinary authored template content after an
   in-body element now reports the required parse error before preserving the
   existing ignored-token DOM behavior, matching WPT `template.dat`'s
   `<template><div><tr>` evidence. Valid template table sequences, nested
   templates, real tables, foreign content, ordinary outside-template content,
   and synthetic template fragments remain on their existing paths. The same
   parse error is now reported when the intervening in-body element has already
   closed and the authored template is current again, while the row remains
   ignored. The adjacent `caption`, `col`, `colgroup`, table-section, and cell
   starts now report the same in-body family parse error and remain ignored
   after an authored template has entered body mode, both within the body
   descendant and after the template becomes current again. Valid direct
   template table transitions, real tables, nested templates, foreign content,
   ordinary outside-table handling, and synthetic template fragments remain on
   their existing paths. The distinct in-body `frame` and `head` starts were
   re-audited and already emit their required parse errors while remaining
   ignored through the existing general frame/head recovery. The adjacent
   authored-template `head` and `frameset` end tags now report their in-body
   parse error while preserving ignored-token DOM behavior. Direct and nested
   authored templates are covered; real head/frameset closure, real tables,
   ordinary stray endings, foreign content, and synthetic template fragments
   remain on their existing paths. A `caption` end tag rejected after a
   template-owned cell now reports the in-row parse error while remaining
   ignored, matching WPT `template.dat`'s template cell/caption and
   cell/column-group sequences. Matching real captions, nested templates,
   foreign content, ordinary stray endings, and synthetic template fragments
   remain on their existing paths. Continue auditing the remaining template
   state boundaries. A `tr`
   start after non-whitespace template text now follows the still-active
   template insertion mode into the table-body row transition instead of being
   silently discarded, preserving the authored text before the row. Whitespace,
   nested templates, foreign content, synthetic template fragments, valid
   direct rows, and the rejected in-body row path retain their expected
   behavior. Authored-template `<html>`, `<body>`, and `<frameset>` starts now
   report the in-body shell-start parse error before remaining ignored. The
   ignored `<body>` token also no longer marks an explicit body start and
   incorrectly disables a later valid frameset. Ordinary document-shell starts,
   nested templates, foreign template-named elements, and synthetic template
   fragment contexts remain on their existing paths. The remaining
   template-state branches are now audited against the current Standard and
   live corpus without another uncovered diagnostic boundary.
   Seeded table and foreign fragment-shell boundaries now report their required
   parse errors.
2. **Adoption agency and active formatting.** Cover malformed formatting cases
   without changing their now-conforming DOM output. Matching open formatting
   elements that are not current now report before existing DOM repair, as do
   matching end tags whose formatting element was displaced from the open stack
   before reconstruction inside a paragraph boundary. When a newer same-name
   active entry has left the stack, its end tag now reports and removes that
   entry instead of silently selecting an older open formatting element,
   covering both `tests1.dat` adoption-agency-1.2 shapes. Adoption-agency end tags
   reprocessed from foreign content now report both the foreign mismatch and
   non-current formatting errors while preserving the conforming foreign DOM,
   matching WPT `adoption01.dat`'s `<a><svg><tr><input></a>` evidence. Repeated
   HTML anchor start tags now report before the existing adoption
   recovery and preserve both anchors required by WPT `adoption02.dat`'s
   `<a><div><style></style><address><a>` row. Repeated anchors reprocessed
   through table foster parenting now additionally report both the repeated
   start and out-of-table-scope adoption errors declared by `tests1.dat` and
   `template.dat`, while preserving the existing fostered DOM. Div-based
   adoption recovery now reports the non-current formatting error for each
   outer-loop iteration represented by the parser's combined repair, matching
   the two same-token `adoption-agency-1.3` declarations in WPT `tests8.dat`
   and `tests19.dat` while preserving document, table-cell, and fragment DOM
   output. Formatting end tags whose older matching element is hidden behind
   an open `applet`, `marquee`, or `object` marker now remain on generic
   non-current end-tag recovery instead of emitting an adoption-agency error,
   matching WPT `tests1.dat`'s marquee boundary rows while preserving marker,
   foster-parenting, and fragment DOM output. Repeated `nobr` starts now
   report when an authored HTML `nobr` is in scope before preserving the
   existing adoption recovery, covering WPT `tests26.dat`'s ordinary,
   reconstructed, and foster-parented rows while keeping first starts, open
   marker boundaries, foreign breakout routing, and synthetic fragment
   contexts on their required paths. Noah's Ark reconstruction now treats
   attribute order as insignificant when limiting equivalent formatting
   entries to the three most recent entries, while preserving entries with
   distinct attribute values and marker-isolated formatting. Div-based
   adoption recovery now applies the inner-loop replacement limit to the three
   stack nodes nearest the furthest block, counts intervening non-formatting
   nodes, and preserves active formatting opened below that block. This covers
   the deep `adoption01.dat` formatting shape with follow-on text while keeping
   the three-node boundary and existing table reconstruction conforming. Nested
   div recovery now also remaps the repaired open stack through each formatting
   clone inserted before the selected descendant, so follow-on text and
   formatting remain at the innermost div in document, table-cell, and fragment
   contexts. Mixed active/non-active wrapper recovery now carries that same
   remapping through nested divs, preserving the furthest block and current
   node when a non-formatting wrapper separates retained formatting entries.
   Bookmark placement and active-formatting replacement were re-audited with
   follow-on formatting starts, ends, markers, same-name entries,
   non-formatting separators, and nested blocks across 256 generated
   current-browser differentials without another uncovered boundary. Repeated
   `button` starts now report when an authored HTML button is in button scope
   before preserving the existing implied closure, matching WPT `tests6.dat`
   and `tests20.dat`. First buttons in ordinary and real-cell contexts,
   already-closed buttons, marker-separated buttons, and synthetic button
   fragment contexts remain quiet. A heading start tag now reports when its
   current node is another heading before popping that heading, matching WPT
   `tests1.dat`'s `<h1><h2>` rows. A non-current heading separated by an inline
   or special element remains open, matching the current Standard and browser
   behavior. Ruby annotation starts now generate the required implied end tags
   when an authored HTML `ruby` is in scope and report when a non-ruby node
   remains current, matching WPT `tests19.dat` and `webkit01.dat` while keeping
   valid annotation transitions, outside-ruby starts, and synthetic ruby
   fragment contexts quiet. An `hr` start tag in select scope now generates
   implied ends and reports when a non-implied descendant leaves an `option` or
   `optgroup` open, matching the current Standard while inserting the `hr` at
   the surviving current node. Direct option and group children close quietly;
   empty selects, ordinary body content, table cells, select fragments, and
   foreign content retain their expected paths. Continue the fresh in-body
   scope and recovery audit beyond active formatting. Template-owned `form`
   elements now bypass an outer form pointer without replacing or clearing it,
   matching the current Standard while ordinary repeated forms remain ignored
   and foreign template-named elements do not activate the HTML-template path.
   Form end tags now report when implied-end-tag generation leaves a non-form
   node current before the form is removed from the stack, matching WPT
   `tests6.dat` while preserving current-form and implied-descendant closures.
   Form end tags whose pointer-owned form is blocked by table scope now report
   before preserving the existing pointer clearing and DOM recovery, matching
   WPT `tests16.dat`; template-owned forms blocked by the same boundary are
   covered, while ordinary, foreign-content, cell, and fragment paths keep
   their existing behavior.
3. **Diagnostic positions and error taxonomy.** Carry source positions into
   tree construction and map diagnostics to current WHATWG concepts. Legacy
   WPT/html5lib error labels are evidence hints, not a normative public API.
4. **Input boundary review.** Document the Unicode-code-point parser boundary
   and either add or explicitly separate byte decoding and encoding sniffing.
5. **Algorithm and differential audit.** Map implemented states/modes to the
   current HTML Standard and add deterministic differential/fuzz coverage for
   branches not exercised by the upstream corpora.

## Intake Order

When a fresh upstream audit discovers work, add each bounded item here before
starting the next pull request and prioritize it in this order:

1. Missing executable WPT tree-construction cases.
2. Missing or skipped html5lib tokenizer cases.
3. Missing required tokenizer or tree-construction diagnostics.
4. Public DOM model gaps required by an upstream case.
5. Input-boundary or algorithm-coverage gaps.
6. Audit, fixture, or documentation drift that weakens reproducibility.

Keep one item per pull request. After each merge, rerun the upstream audit,
record newly discovered items, reprioritize this queue, and select the highest
remaining item.
